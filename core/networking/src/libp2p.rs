use futures::{channel::{mpsc::{self, channel, Receiver}, oneshot}, executor::block_on, lock::Mutex, AsyncWriteExt, SinkExt, StreamExt, FutureExt};
use libp2p::{core::muxing::StreamMuxerBox, dcutr, gossipsub::{self, IdentTopic, SubscriptionError}, identity::ParseError, kad::{self, store::MemoryStore, GetClosestPeersOk, ProgressStep, QueryId}, multiaddr::{Multiaddr, Protocol}, noise, ping, swarm::{behaviour::toggle::Toggle, DialError, InvalidProtocol, NetworkBehaviour, SwarmEvent}, yamux, PeerId, Stream, StreamProtocol, Swarm, Transport, TransportError};
use posemesh_utils::retry_with_delay;
use std::{collections::HashMap, error::Error, fmt::{self, Debug, Formatter}, io::{self, Read, Write}, str::FromStr, sync::Arc, time::Duration};
use rand::{rngs::OsRng, thread_rng};
use libp2p_stream::{self as stream, AlreadyRegistered, IncomingStreams, OpenStreamError};
use crate::{client::{self, Client}, event};
use std::net::{Ipv4Addr, IpAddr};

#[cfg(not(target_family="wasm"))]
use libp2p_webrtc as webrtc;
#[cfg(not(target_family="wasm"))]
use libp2p::{tcp, mdns};
#[cfg(not(target_family="wasm"))]
use std::{fs, path::Path};

#[cfg(not(target_family="wasm"))]
use tokio::spawn;
#[cfg(target_family="wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

#[cfg(not(target_family="wasm"))]
use tokio::time::sleep;

#[cfg(target_family="wasm")]
use libp2p_webrtc_websys as webrtc_websys;
#[cfg(target_family="wasm")]
use libp2p_websocket_websys as ws_websys;
#[cfg(target_family="wasm")]
use libp2p::core::transport::upgrade::Version;

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Address in use")]
    AddressInUse,
    #[error("Transport error: {0}")]
    TransportError(#[from] TransportError<std::io::Error>),
    #[error("Swarm initialization failed: {0}")]
    SwarmInitializationFailed(Box<dyn Error + Send + Sync>),
    #[error("Dial error: {0}")]
    DialError(#[from] DialError),
    #[error("Stream error: {0}")]
    StreamError(#[from] io::Error),
    #[error("Open stream error: {0}")]
    OpenStreamError(#[from] OpenStreamError),
    #[error("Already registered")]
    AlreadyRegistered(#[from] AlreadyRegistered),
    #[error("Event sender error")]
    EventSenderError(#[from] mpsc::TrySendError<event::Event>),
    #[error("Gossipsub error: {0}")]
    GossipsubError(#[from] gossipsub::SubscriptionError),
    #[error("Channel error: {0}")]
    ChannelSendError(#[from] mpsc::SendError),
    #[error("Channel receiver error: {0}")]
    ChannelReceiverError(#[from] oneshot::Canceled),
    #[error("Invalid protocol: {0}")]
    InvalidProtocol(#[from] InvalidProtocol),
    #[error("Parse error: {0}")]
    ParseError(#[from] ParseError),
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => !ipv4.is_private() && !ipv4.is_loopback(),
        IpAddr::V6(ipv6) => !ipv6.is_loopback() && !ipv6.is_unspecified(),
    }
}

/// Checks if an IP address is publicly routable
fn is_public(addr: Multiaddr) -> bool {
    for proto in addr.iter() {
        if let Protocol::Ip4(ip) = proto {
            return is_public_ip(IpAddr::V4(ip));
        }
        if let Protocol::Ip6(ip) = proto {
            return is_public_ip(IpAddr::V6(ip));
        }
    }
    false
}

fn is_circuit_addr(addr: Multiaddr) -> bool {
    for proto in addr.iter() {
        if let Protocol::P2pCircuit = proto {
            return true;
        }
    }
    false
}

fn is_webrtc_addr(addr: Multiaddr) -> bool {
    for proto in addr.iter() {
        if let Protocol::WebRTCDirect = proto {
            return true;
        }
    }
    false
}
// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
struct PosemeshBehaviour {
    gossipsub: gossipsub::Behaviour,
    streams: stream::Behaviour,
    identify: libp2p::identify::Behaviour,
    kdht: Toggle<libp2p::kad::Behaviour<MemoryStore>>,
    autonat_client: Toggle<libp2p::autonat::v2::client::Behaviour>,
    relay_client: Toggle<libp2p::relay::client::Behaviour>,
    ping: libp2p::ping::Behaviour,
    #[cfg(not(target_family="wasm"))]
    mdns: Toggle<mdns::tokio::Behaviour>,
    #[cfg(not(target_family="wasm"))]
    relay: Toggle<libp2p::relay::Behaviour>,
    #[cfg(not(target_family="wasm"))]
    autonat_server: Toggle<libp2p::autonat::v2::server::Behaviour>,
    dcutr: Toggle<libp2p::dcutr::Behaviour>,
}

#[derive(Clone)]
pub struct NetworkingConfig {
    pub enable_relay_server: bool,
    pub port: u16,
    pub bootstrap_nodes: Vec<String>,
    pub relay_nodes: Vec<String>,
    pub enable_mdns: bool,
    pub private_key: Option<Vec<u8>>,
    pub private_key_path: Option<String>,
    pub enable_kdht: bool,
    pub enable_websocket: bool,
    pub enable_webrtc: bool,
    pub namespace: Option<String>,
}

impl Default for NetworkingConfig {
    fn default() -> Self {
        NetworkingConfig{
            port: 0,
            bootstrap_nodes: vec![],
            enable_relay_server: false,
            enable_kdht: false,
            enable_mdns: true,
            relay_nodes: vec![],
            private_key: None,
            private_key_path: Some("/volume/pkey".to_string()),
            enable_webrtc: false,
            enable_websocket: false, // placeholder
            namespace: None,
        }
    }
}

fn protocol(namespace: Option<String>, protocol: &str) -> StreamProtocol {
    if let Some(ns) = namespace {
        StreamProtocol::try_from_owned(format!("/posemesh/cluster/{}/{}", ns, protocol)).unwrap()
    } else {
        StreamProtocol::try_from_owned(format!("/posemesh/{}", protocol)).unwrap()
    }
}

struct Libp2p {
    swarm: Swarm<PosemeshBehaviour>,
    cfg: NetworkingConfig,
    command_receiver: mpsc::Receiver<client::Command>,
    pub node_id: String,
    event_sender: mpsc::Sender<event::Event>,
    find_peer_requests: Arc<Mutex<HashMap<QueryId, oneshot::Sender<Result<(), NetworkError>>>>>,
    cancel_sender: Option<oneshot::Sender<()>>,
}

#[cfg(not(target_family="wasm"))]
fn keypair_file(private_key_path: &String) -> libp2p::identity::Keypair {
    let path = Path::new(private_key_path);
    // Check if the keypair file exists
    if let Ok(mut file) = fs::File::open(path) {
        // Read the keypair from the file
        let mut keypair_bytes = Vec::new();
        if file.read_to_end(&mut keypair_bytes).is_ok() {
            if let Ok(keypair) = libp2p::identity::Keypair::from_protobuf_encoding(&keypair_bytes) {
                return keypair;
            }
        }
    }

    // If the file does not exist or reading failed, create a new keypair
    let keypair = libp2p::identity::Keypair::generate_ed25519();

    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            tracing::error!("Failed to create directory: {err}");
        }
    }

    // Save the new keypair to the file
    if let Ok(mut file) = fs::File::create(path) {
        let keypair_bytes = keypair.to_protobuf_encoding().expect("Failed to encode keypair");
        if file.write_all(&keypair_bytes).is_err() {
            tracing::error!("Failed to write keypair to file");
        }
    }

    keypair
}

fn parse_or_create_keypair(
    private_key: Option<Vec<u8>>,
    private_key_path: Option<String>,
) -> libp2p::identity::Keypair {
    let private_key = private_key.unwrap_or_default();
    // load private key into keypair
    if let Ok(keypair) = libp2p::identity::Keypair::ed25519_from_bytes(private_key) {
        return keypair;
    }

    #[cfg(not(target_family="wasm"))]
    if let Some(key_path) = private_key_path {
        return keypair_file(&key_path);
    }

    libp2p::identity::Keypair::generate_ed25519()
}

async fn build_swarm(key: libp2p::identity::Keypair, mut behavior: PosemeshBehaviour) -> Result<Swarm<PosemeshBehaviour>, Box<dyn Error + Send + Sync>> {
    #[cfg(not(target_family="wasm"))]
    let swarm = libp2p::SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_other_transport(|id_keys| {
            Ok(webrtc::tokio::Transport::new(
                id_keys.clone(),
                webrtc::tokio::Certificate::generate(&mut thread_rng())?,
            )
            .map(|(peer_id, conn), _| (peer_id, StreamMuxerBox::new(conn))))
        })?
        .with_dns()?
        .with_websocket(
            noise::Config::new,
            yamux::Config::default,
        ).await?
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|_, relay_behavior| {
            behavior.relay_client = Some(relay_behavior).into();
            behavior
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    #[cfg(target_family="wasm")]
    let swarm = libp2p::SwarmBuilder::with_existing_identity(key)
        .with_wasm_bindgen()
        .with_other_transport(|key| {
            webrtc_websys::Transport::new(webrtc_websys::Config::new(&key))
        })?
        .with_other_transport(|key| {
            Ok(ws_websys::Transport::default()
            .upgrade(Version::V1Lazy)
            .authenticate(noise::Config::new(&key).expect("Failed to create noise config"))
            .multiplex(yamux::Config::default()))
        })?
        .with_behaviour(|_| behavior)?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    Ok(swarm)
}

fn build_behavior(key: libp2p::identity::Keypair, cfg: &NetworkingConfig) -> PosemeshBehaviour {
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .message_id_fn(|message: &gossipsub::Message| {
            gossipsub::MessageId::from(format!("{}-{:?}", String::from_utf8_lossy(&message.data), message.sequence_number.unwrap()))
        })
        .build()
        .expect("Failed to build gossipsub config");

    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(key.clone()),
        gossipsub_config,
    )
    .expect("Failed to build gossipsub behaviour");

    let streams = stream::Behaviour::new();
    let identify = libp2p::identify::Behaviour::new(
        libp2p::identify::Config::new(protocol(cfg.namespace.clone(), "id/1.0.0").to_string(), key.public())
        .with_push_listen_addr_updates(true),
    );

    let mut behavior = PosemeshBehaviour {
        gossipsub,
        streams,
        identify,
        autonat_client: None.into(),
        relay_client: None.into(),
        kdht: None.into(),
        #[cfg(not(target_family="wasm"))]
        mdns: None.into(),
        #[cfg(not(target_family="wasm"))]
        relay: None.into(),
        #[cfg(not(target_family="wasm"))]
        autonat_server: None.into(),
        dcutr: None.into(),
        ping: libp2p::ping::Behaviour::default()
    };

    #[cfg(not(target_family="wasm"))]
    if cfg.enable_mdns {
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
            .expect("Failed to build mdns behaviour");
        behavior.mdns = Some(mdns).into();
    }
    
    #[cfg(not(target_family="wasm"))]
    if cfg.enable_relay_server {
        let mut relay_config = libp2p::relay::Config::default();
        relay_config.max_circuit_bytes = 1024 * 1024 * 1024; // 1GB
        let relay = libp2p::relay::Behaviour::new(key.public().to_peer_id(), relay_config);
        behavior.relay = Some(relay).into();
        behavior.autonat_server = Some(libp2p::autonat::v2::server::Behaviour::new(OsRng)).into();
    } else {
        behavior.autonat_client = Some(libp2p::autonat::v2::client::Behaviour::new(OsRng,libp2p::autonat::v2::client::Config::default())).into();
        behavior.dcutr = Some(libp2p::dcutr::Behaviour::new(key.public().to_peer_id())).into();
    }

    if cfg.enable_kdht {
        let mut kad_cfg = libp2p::kad::Config::new(protocol(cfg.namespace.clone(), "kad/1.0.0"));
        kad_cfg.set_query_timeout(Duration::from_secs(5));
        let store = libp2p::kad::store::MemoryStore::new(key.public().to_peer_id());
        let mut kdht = libp2p::kad::Behaviour::with_config(key.public().to_peer_id(), store, kad_cfg);

        if cfg.enable_relay_server {
            kdht.set_mode(Some(kad::Mode::Server));
        } else {
            kdht.set_mode(Some(kad::Mode::Client));
        }
        
        let bootstrap_nodes = cfg.bootstrap_nodes.clone();
        for bootstrap in bootstrap_nodes {
            let peer_id = match bootstrap.split('/').last() {
                Some(peer_id) => PeerId::from_str(peer_id).unwrap(),
                None => continue,
            };
            let maddr = Multiaddr::from_str(&bootstrap).expect("Failed to parse bootstrap node address");
            let _ = kdht.add_address(&peer_id, maddr);
        }

        behavior.kdht = Some(kdht).into();
    }

    behavior
}

fn build_listeners(port: u16) -> Vec<Multiaddr> {
    #[cfg(not(target_family="wasm"))]
    return vec![
        Multiaddr::from(Ipv4Addr::UNSPECIFIED)
            .with(Protocol::Tcp(port)),
        Multiaddr::from(Ipv4Addr::UNSPECIFIED)
            .with(Protocol::Udp(port))
            .with(Protocol::QuicV1),
    ];

    #[cfg(target_family="wasm")]
    return vec![];
}

fn enable_websocket(port: u16) -> Multiaddr {
    Multiaddr::from(Ipv4Addr::UNSPECIFIED)
        .with(Protocol::Tcp(port))
        .with(Protocol::Ws("/".into()))
}

fn enable_webrtc(port: u16) -> Multiaddr {
    let mut webrtc_port = port + 1;
    if port == 0 {
        webrtc_port = 0;
    }
    Multiaddr::from(Ipv4Addr::UNSPECIFIED)
        .with(Protocol::Udp(webrtc_port))
        .with(Protocol::WebRTCDirect)
}

impl Libp2p {
    pub async fn new(cfg: &NetworkingConfig, command_receiver: mpsc::Receiver<client::Command>, event_sender: mpsc::Sender<event::Event>) -> Result<String, NetworkError> {
        let private_key = cfg.private_key.clone();
        let key = parse_or_create_keypair(private_key, cfg.private_key_path.clone());

        let behaviour = build_behavior(key.clone(), cfg);

        let mut swarm = build_swarm(key.clone(), behaviour).await.map_err(|e| NetworkError::SwarmInitializationFailed(e))?;

        let mut listeners = build_listeners(cfg.port);
        if cfg.enable_websocket {
            listeners.push(enable_websocket(cfg.port));
        }
        if cfg.enable_webrtc {
            listeners.push(enable_webrtc(cfg.port));
        }
        
        for addr in listeners.iter() {
            match swarm.listen_on(addr.clone()) {
                Ok(_) => {},
                Err(e) => {
                    match e {
                        TransportError::MultiaddrNotSupported(_) => {
                            return Err(NetworkError::TransportError(e));
                        }
                        TransportError::Other(ie) => {
                            #[cfg(any(target_os = "macos", target_os = "ios", target_os = "tvos", target_os = "watchos"))]
                            tracing::warn!("Failed to initialize networking: Apple platforms require 'com.apple.security.network.server' entitlement set to YES.");
                        
                            if ie.kind() == std::io::ErrorKind::AddrInUse {
                                return Err(NetworkError::AddressInUse);
                            } else {
                                return Err(NetworkError::TransportError(TransportError::Other(ie)));
                            }
                        }
                    }
                }
            }
        }
        let (cancel_sender, cancel_receiver) = oneshot::channel::<()>();
        let mut cancel_receiver = cancel_receiver.fuse();

        let node_id = key.public().to_peer_id();
        let networking = Libp2p {
            cfg: cfg.clone(),
            swarm: swarm,
            command_receiver: command_receiver,
            node_id: node_id.to_string(),
            event_sender: event_sender,
            find_peer_requests: Arc::new(Mutex::new(HashMap::new())),
            cancel_sender: Some(cancel_sender),
        };
        
        spawn(async move {
            futures::select! {
                _ = cancel_receiver => {
                    tracing::info!("Cancelling networking");
                    return;
                }
                _ = networking.run().fuse() => {
                    tracing::info!("Networking complete");
                }
            }
        });

        Ok(node_id.to_string())
    }

    async fn run(mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        tracing::info!("Starting networking");
        
        #[cfg(not(target_family="wasm"))]
        loop {
            tokio::select! {
                Some(event) = self.swarm.next() => self.handle_event(event).await,
                Some(command) = self.command_receiver.next() => self.handle_command(command).await,
                else => break,
            }
        };

        #[cfg(target_family="wasm")]
        loop {
            futures::select! {
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                command = self.command_receiver.select_next_some() => self.handle_command(command).await,
                complete => break,
            }
        };

        Ok(())
    }
    
    async fn handle_event(&mut self, event :SwarmEvent<PosemeshBehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Dcutr(dcutr::Event {
                remote_peer_id,
                result: Ok(_),
                ..  
            })) => {
                tracing::info!("Successfully hole-punched to {remote_peer_id}");
            }
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Dcutr(dcutr::Event {
                remote_peer_id,
                result: Err(e),
                ..  
            })) => {
                tracing::error!("Failed to hole-punch to {remote_peer_id}: {e}");
            }
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Kdht(
                kad::Event::OutboundQueryProgressed {
                    id,
                    result: kad::QueryResult::GetClosestPeers(Ok(GetClosestPeersOk { key, peers, .. })),
                    step: ProgressStep { count, last },
                    ..
                }
            )) => {
                let peer_id_res = PeerId::from_bytes(key.as_slice());
                if peer_id_res.is_err() {
                    tracing::error!("Failed to convert key to peer id");
                    return;
                }
                let peer_id = peer_id_res.unwrap();
                tracing::info!("GetClosestPeersOk Got {:?} peer(s) for {:#}, count {:?}, last {:?}", peers.len(), peer_id, count, last);
                let mut found_address = false;
                for peer in peers {
                    found_address = !peer.addrs.is_empty();
                    self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer.peer_id);
                    for addr in peer.addrs {
                        tracing::info!("Adding address to DHT: {}", addr.clone());
                        self.swarm.behaviour_mut().kdht.as_mut().map(|dht| {
                            dht.add_address(&peer.peer_id, addr.clone());
                        });
                    }
                }
                let mut find_peer_requests =  self.find_peer_requests.lock().await;
                if find_peer_requests.contains_key(&id) {
                    let sender = find_peer_requests.remove(&id);
                    if sender.is_none() {
                        return;
                    }
                    if found_address {
                        let _ = sender.unwrap().send(Ok(()));
                    } else {
                        let _ = sender.unwrap().send(Err(NetworkError::DialError(DialError::NoAddresses)));
                    }
                } else if last {
                    tracing::warn!("No request found for peer: {peer_id}");
                }
            }
            // get closest peers err
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Kdht(
                kad::Event::OutboundQueryProgressed {
                    result: kad::QueryResult::GetClosestPeers(Err(e)),
                    ..
                }
            )) => {
                tracing::error!("GetClosestPeers failed: {e}");
            }
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Kdht(
                kad::Event::OutboundQueryProgressed {
                    result: kad::QueryResult::Bootstrap(Err(e)),
                    ..
                }
            )) => {
                tracing::error!("Bootstrap failed: {e}");
            }
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Kdht(
                kad::Event::OutboundQueryProgressed {
                    result: kad::QueryResult::Bootstrap(Ok(_)),
                    ..
                },
            )) => {
                tracing::info!("Bootstrap succeeded");
            }
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Kdht(_)) => {
                tracing::info!("KDHT event => {event:?}");
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                let local_peer_id = *self.swarm.local_peer_id();
                let address = address.clone().with(Protocol::P2p(local_peer_id));
                self.event_sender.send(event::Event::NewAddress { address: address.clone() }).await.expect("failed to send new address");
                println!("Local node is listening on {:?}", address);
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                tracing::info!("Connected to {peer_id} on {:?}", endpoint.get_remote_address());
                self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            }
            SwarmEvent::Dialing {
                peer_id: Some(peer_id),
                ..
            } => tracing::info!("Dialing {peer_id}"),
            #[cfg(not(target_family="wasm"))]
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer_id, _multiaddr) in list {
                    tracing::info!("mDNS discovered a new peer: {peer_id}");
                    self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                }
            },
            #[cfg(not(target_family="wasm"))]
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                for (peer_id, _multiaddr) in list {
                    tracing::info!("mDNS discover peer has expired: {peer_id}");
                    self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                }
            },
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                message: gossipsub::Message { source, data, topic, .. },
                ..
            })) => {
                if let Err(e) = self.event_sender.send(event::Event::PubSubMessageReceivedEvent { 
                        topic: topic.clone(),
                        message: data.clone(),
                        from: source,
                    }).await {
                    tracing::error!("Failed to send pubsub message: {e}");
                }
            },
            // Prints peer id identify info is being sent to.
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Identify(libp2p::identify::Event::Sent { peer_id, .. })) => {
                tracing::info!("Sent identify info to {peer_id:?}")
            },
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::AutonatClient(libp2p::autonat::v2::client::Event {
                server,
                tested_addr,
                bytes_sent,
                result: Ok(()),
            })) => {
                tracing::info!("Tested {tested_addr} with {server}. Sent {bytes_sent} bytes for verification. Everything Ok and verified.");
                self.swarm.add_external_address(tested_addr.clone());
            }
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::AutonatClient(libp2p::autonat::v2::client::Event {
                server,
                tested_addr,
                bytes_sent,
                result: Err(e),
            })) => {
                tracing::info!("Tested {tested_addr} with {server}. Sent {bytes_sent} bytes for verification. Failed with {e:?}.");

                for relay in self.cfg.relay_nodes.iter() {
                    let maddr = Multiaddr::from_str(relay).unwrap();
                    let addr = maddr
                        .with(Protocol::P2pCircuit);
                    match self.swarm.listen_on(addr.clone()) {
                        Ok(_) => {
                            tracing::info!("Listening on relay address: {addr}");
                        },
                        Err(e) => {
                            tracing::error!("Failed to listen on relay address: {addr}. Error: {e}");
                        }
                    }
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, connection_id, endpoint, num_established, cause } => {
                if num_established == 0 {
                    self.swarm.behaviour_mut().kdht.as_mut().map(|dht| {
                        dht.remove_peer(&peer_id);
                    });
                    self.event_sender.send(event::Event::NodeUnregistered { node_id: peer_id.to_string() }).await.unwrap_or_else(|_| panic!("{}: Failed to send node unregistered event", self.node_id)); 
                }
                tracing::info!("Connection closed: {peer_id} {connection_id} {endpoint:?} {num_established} {cause:?}");
            }
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Ping(ping::Event { peer, connection, result, .. })) => {
                tracing::info!("Ping {peer} {connection} {result:?}");
                if let Err(e) = result {
                    // Only log the error since ConnectionClosed will handle peer removal
                    tracing::error!("Ping failed for peer {peer}: {e}");
                }
            }
            SwarmEvent::ExternalAddrConfirmed { address } => {
                tracing::info!("External address confirmed: {address}");
                let peer_id = self.swarm.local_peer_id().clone();
                self.swarm.behaviour_mut().kdht.as_mut().map(|dht| {
                    dht.add_address(&peer_id, address.clone());
                    // if !is_circuit_addr(address.clone()) {
                    //     dht.set_mode(Some(kad::Mode::Server));
                    // }
                });
            }
            SwarmEvent::NewExternalAddrCandidate { address } => {
                tracing::info!("New external address candidate: {address}");
            }
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::RelayClient(
                libp2p::relay::client::Event::ReservationReqAccepted { .. },
            )) => {
                tracing::info!("Relay accepted our reservation request");
            }
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::RelayClient(event)) => {
                tracing::info!("Relay Client: {event:?}");
            }
            #[cfg(not(target_family="wasm"))]
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::AutonatServer(libp2p::autonat::v2::server::Event {tested_addr, result, ..})) => {
                tracing::info!("Autonat Server tested address: {tested_addr}, result: {result:?}");
            }
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Identify(libp2p::identify::Event::Received {
                info: libp2p::identify::Info { observed_addr, listen_addrs, .. },
                peer_id,
                ..
            })) =>
            {
                tracing::info!("Observed address: {observed_addr} from {peer_id}. {:?}", listen_addrs);
                if self.cfg.enable_relay_server {
                    self.swarm.add_external_address(observed_addr.clone());
                }
                
                self.swarm.behaviour_mut().kdht.as_mut().map(|dht| {
                    for addr in listen_addrs.clone() {
                        dht.add_address(&peer_id, addr.clone());
                    }
                });
            },
            e => tracing::debug!("Other events: {e:?}"),
        }
    }

    async fn handle_command(&mut self, command: client::Command) {
        match command {
            client::Command::Send { message, peer_id, protocol, response } => {
                let ctrl = self.swarm.behaviour_mut().streams.new_control();
                let mut receiver = None;
                if !Swarm::is_connected(&self.swarm, &peer_id) {
                    tracing::info!("Peer {peer_id} is not connected, trying to find it");
                    receiver = Some(self.find_peer(peer_id).await);
                }
                #[cfg(target_family="wasm")]
                wasm_bindgen_futures::spawn_local(open_stream(ctrl, peer_id, protocol, message, response, receiver));

                #[cfg(not(target_family="wasm"))]
                tokio::spawn(open_stream(ctrl, peer_id, protocol, message, response, receiver));
            },
            client::Command::SetStreamHandler { endpoint, sender } => {
                match self.add_stream_protocol(&endpoint) {
                    Ok(stream) => {
                        let _ = sender.send(Ok(stream));
                    }
                    Err(e) => {
                        let _ = sender.send(Err(e));
                    }
                }
            },
            client::Command::Subscribe { topic, resp } => {
                match self.subscribe(topic) {
                    Ok(_) => {
                        let _ = resp.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = resp.send(Err(e));
                    }
                }
            }
            client::Command::Publish { topic, message, sender } => {
                let t = IdentTopic::new(topic);
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(t, message) {
                    let _ = sender.send(Err(NetworkError::GossipsubError(SubscriptionError::PublishError(e))));
                    return;
                }
                let _ = sender.send(Ok(()));
            }
            client::Command::Cancel { sender } => {
                let Some(cancel_sender) = self.cancel_sender.take() else {
                    tracing::error!("No cancel sender found, it was already cancelled");
                    return;
                };
                let _ = cancel_sender.send(());
                let _ = sender.send(());
            }
        }
    }

    fn subscribe(&mut self, topic: String) -> Result<(), NetworkError> {
        let t = IdentTopic::new(topic);
        
        match self.swarm.behaviour_mut().gossipsub.subscribe(&t) {
            Ok(_) => Ok(()),
            Err(e) => Err(NetworkError::GossipsubError(e)),
        }
    }

    fn add_stream_protocol(&mut self, endpoint: &str) -> Result<IncomingStreams, NetworkError> {
        let proto = StreamProtocol::try_from_owned(endpoint.to_string())?;
        let incoming_stream = self.swarm.behaviour_mut().streams.new_control().accept(proto)?;

        Ok(incoming_stream)
    }

    async fn find_peer(&mut self, peer_id: PeerId) -> oneshot::Receiver<Result<(), NetworkError>> {
        let (sender, receiver) = oneshot::channel::<Result<(), NetworkError>>();
        let mut find_peer_requests_lock = self.find_peer_requests.lock().await;

        if let Some(kdht) = self.swarm.behaviour_mut().kdht.as_mut() {
            let q = kdht.get_closest_peers(peer_id);
            find_peer_requests_lock.insert(q, sender);
        }

        receiver
    }
}

async fn _open_stream(mut ctrl: stream::Control, peer_id: PeerId, protocol: StreamProtocol, message: Vec<u8>) -> Result<Stream, NetworkError> {
    let mut s = ctrl.open_stream(peer_id, protocol).await?;

    if !message.is_empty() {
        match s.write(&message[..1]).await {
            Ok(0) => {
                return Err(NetworkError::StreamError(io::Error::new(io::ErrorKind::ConnectionReset, "Connection reset")));
            }
            Ok(_) => {
                s.write_all(&message[1..]).await?;
            }
            Err(e) => {
                return Err(NetworkError::StreamError(e));
            }
        }
        s.flush().await?;
    }
    Ok(s)
}

async fn open_stream(ctrl: stream::Control, peer_id: PeerId, protocol: StreamProtocol, message: Vec<u8>, send_response: oneshot::Sender<Result<Stream, NetworkError>>, find_peer_receiver: Option<oneshot::Receiver<Result<(), NetworkError>>>) {
    if let Some(receiver) = find_peer_receiver {
        match receiver.await {
            Ok(Ok(_)) => {
                tracing::debug!("Peer found");
            }
            Ok(Err(e)) => {
                if let Err(e) = send_response.send(Err(e)) {
                    tracing::error!("Failed to send feedback: {:?}", e);
                }
                return;
            }
            Err(_) => {
                tracing::debug!("Cancelled finding peer");
            }
        }
    }
    let s = retry_with_delay(|| Box::pin(_open_stream(ctrl.clone(), peer_id, protocol.clone(), message.clone())), 3, Duration::from_secs(5)).await;
    if let Err(e) = s {
        tracing::error!("Failed to open stream: {:?}", e);
        if let Err(send_err) = send_response.send(Err(e)) {
            tracing::error!("Failed to send feedback: {:?}", send_err);
        }
        return;
    }

    if let Err(send_err) = send_response.send(Ok(s.unwrap())) {
        tracing::error!("Failed to send feedback: {:?}", send_err);
    }
}


#[derive(Clone)]
pub struct Networking {
    pub client: Client,
    pub event_receiver: Arc<Mutex<Receiver<event::Event>>>,
    pub id: String,
}

impl Debug for Networking {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Networking")
            .field("id", &self.id)
            .finish()
    }
}

async fn initialize_libp2p(cfg: &NetworkingConfig, receiver: mpsc::Receiver<client::Command>, event_sender: mpsc::Sender<event::Event>) -> Result<String, NetworkError> {
    let res = Libp2p::new(cfg, receiver, event_sender).await;
    match res {
        Ok(node) => Ok(node),
        Err(e) => Err(e),
    }
}

impl Networking {
    pub fn new(cfg: &NetworkingConfig) -> Result<Self, NetworkError> {
        let (sender, receiver) = channel::<client::Command>(8);
        let (event_sender, event_receiver) = channel::<event::Event>(1072);
        let cfg = cfg.clone();
        let client = Client::new(sender);
        
        let id_res = block_on(initialize_libp2p(&cfg, receiver, event_sender));

        let id = match id_res {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to initialize libp2p: {:?}", e);
                return Err(e);
            }
        };

        Ok(Networking {
            client,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            id,
        })
    }
}

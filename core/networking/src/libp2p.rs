use futures::{channel::{mpsc::{self, channel, Receiver}, oneshot}, lock::Mutex, AsyncWriteExt, SinkExt, StreamExt};
use libp2p::{core::{muxing::StreamMuxerBox, upgrade::Version}, dcutr, yamux, noise, gossipsub::{self, IdentTopic}, kad::{self, store::MemoryStore, GetClosestPeersOk, ProgressStep, QueryId}, multiaddr::{Multiaddr, Protocol}, swarm::{behaviour::toggle::Toggle, DialError, NetworkBehaviour, SwarmEvent}, PeerId, Stream, StreamProtocol, Swarm, Transport};
use utils::retry_with_delay;
use std::{collections::HashMap, error::Error, fmt::{self, Debug, Formatter}, io::{self, Read, Write}, str::FromStr, sync::Arc, time::Duration};
use rand::{thread_rng, rngs::OsRng};
use serde::{Deserialize, Serialize};
use libp2p_stream::{self as stream, IncomingStreams};
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

use futures::executor::block_on;

#[cfg(target_family="wasm")]
use libp2p_webrtc_websys as webrtc_websys;
#[cfg(target_family="wasm")]
use libp2p_websocket_websys as ws_websys;

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
    pub name: String,
    pub enable_websocket: bool,
    pub enable_webrtc: bool,
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
            name: "Placeholder".to_string(),
            enable_webrtc: false,
            enable_websocket: false, // placeholder
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<String>
}

const POSEMESH_PROTO_NAME: StreamProtocol = StreamProtocol::new("/posemesh/kad/1.0.0");

struct Libp2p {
    // nodes_map: HashMap<String, Node>,
    swarm: Swarm<PosemeshBehaviour>,
    cfg: NetworkingConfig,
    command_receiver: mpsc::Receiver<client::Command>,
    pub node: Node,
    // node_regsiter_topic: IdentTopic,
    event_sender: mpsc::Sender<event::Event>,
    find_peer_requests: Arc<Mutex<HashMap<QueryId, oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>>>>,
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
        libp2p::identify::Config::new("/posemesh/id/1.0.0".to_string(), key.public())
        .with_agent_version(cfg.name.clone())
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
        let mut kad_cfg = libp2p::kad::Config::new(POSEMESH_PROTO_NAME);
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
        Multiaddr::empty()
            .with(Protocol::Ip4(Ipv4Addr::UNSPECIFIED))
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
    pub async fn new(cfg: &NetworkingConfig, command_receiver: mpsc::Receiver<client::Command>, event_sender: mpsc::Sender<event::Event>) -> Result<Node, Box<dyn Error + Send + Sync>> {
        let private_key = cfg.private_key.clone();
        let key = parse_or_create_keypair(private_key, cfg.private_key_path.clone());
        println!("Your Peer Id: {:?}", key.public().to_peer_id());

        let behaviour = build_behavior(key.clone(), cfg);

        let mut swarm = build_swarm(key.clone(), behaviour).await?;

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
                    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "tvos", target_os = "watchos"))]
                    eprintln!("Failed to initialize networking: Apple platforms require 'com.apple.security.network.server' entitlement set to YES.");
                    return Err(Box::new(e));
                }
            }
        }
        
        let node = Node {
            id: key.public().to_peer_id().to_string(),
            name: cfg.name.clone(),
            capabilities: vec![],
        };

        let networking = Libp2p {
            cfg: cfg.clone(),
            // nodes_map: nodes_map,
            swarm: swarm,
            command_receiver: command_receiver,
            node: node.clone(),
            // node_regsiter_topic: topic,
            event_sender: event_sender,
            find_peer_requests: Arc::new(Mutex::new(HashMap::new())),
        };

        spawn(async move {
            let _ = networking.run().await;
        });

        Ok(node)
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
                        let _ = sender.unwrap().send(Err(Box::new(DialError::NoAddresses)));
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
                println!(
                    "Local node is listening on {:?}",
                    address.with(Protocol::P2p(local_peer_id))
                );
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
            SwarmEvent::ExternalAddrConfirmed { address } => {
                tracing::info!("External address confirmed: {address}");
                let peer_id = self.swarm.local_peer_id().clone();
                self.swarm.behaviour_mut().kdht.as_mut().map(|dht| {
                    dht.add_address(&peer_id, address.clone());
                    if !is_circuit_addr(address.clone()) {
                        dht.set_mode(Some(kad::Mode::Server));
                    }
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
                info: libp2p::identify::Info { observed_addr, listen_addrs, protocols, agent_version, .. },
                peer_id,
                ..
            })) =>
            {
                tracing::info!("Observed address: {observed_addr} from {peer_id}. {:?}", listen_addrs);
                if self.cfg.enable_relay_server {
                    self.swarm.add_external_address(observed_addr.clone());
                }
                
                self.swarm.behaviour_mut().kdht.as_mut().map(|dht| {
                    for addr in listen_addrs {
                        dht.add_address(&peer_id, addr.clone());
                    }
                });

                let node = Node {
                    id: peer_id.to_string(),
                    name: agent_version,
                    capabilities: protocols.iter().map(|p| p.to_string()).filter(|p| !p.contains("posemesh") && !p.contains("libp2p") && !p.contains("ipfs") ).collect::<Vec<String>>(),
                };

                self.event_sender.send(event::Event::NewNodeRegistered { node: node.clone() }).await.unwrap_or_else(|_| panic!("{}: Failed to send new node: {} registered event", self.node.id, node.name));
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
            client::Command::SetStreamHandler { protocol, sender } => {
                self.add_stream_protocol(protocol, sender);
            }
            client::Command::Subscribe { topic, resp } => {
                self.subscribe(topic, resp);
            }
            client::Command::Publish { topic, message, sender } => {
                let t = IdentTopic::new(topic);
                let res = self.swarm.behaviour_mut().gossipsub.publish(t, message);
                if res.is_err() {
                    let _ = sender.send(Err(Box::new(res.err().unwrap())));
                    return;
                }
                let _ = sender.send(Ok(()));
            }
        }
    }

    // fn register_node(self: &mut Self) -> Result<(), Box<dyn Error>> {
    //     let serialized = serde_json::to_vec(&self.node.clone())?;
    //     self.swarm.behaviour_mut().gossipsub.publish(self.node_regsiter_topic.clone(), serialized)?;
    //     Ok(())
    // }

    fn subscribe(&mut self, topic: String, sender: oneshot::Sender<Box<dyn Error + Send + Sync>>) {
        let t = IdentTopic::new(topic);
        
        match self.swarm.behaviour_mut().gossipsub.subscribe(&t) {
            Ok(_) => {},
            Err(e) => {
                let _ = sender.send(Box::new(e));
            }
        } 
    }

    fn add_stream_protocol(&mut self, protocol: StreamProtocol, sender: oneshot::Sender<Result<IncomingStreams, Box<dyn Error + Send + Sync>>>) {
        let proto = protocol.clone();
        let protocol_ctrl = self.swarm.behaviour_mut().streams.new_control().accept(protocol);
        if protocol_ctrl.is_err() {
            let _ = sender.send(Err(Box::new(protocol_ctrl.err().unwrap())));
            return;
        }
        let incoming_stream = protocol_ctrl.unwrap();

        let mut node = self.node.clone();

        node.capabilities.push(proto.to_string());

        self.node = node;
        self.event_sender.try_send(event::Event::NewNodeRegistered { node: self.node.clone() }).unwrap();

        let _ = sender.send(Ok(incoming_stream));
    }

    async fn find_peer(&mut self, peer_id: PeerId) -> oneshot::Receiver<Result<(), Box<dyn Error + Send + Sync>>> {
        let (sender, receiver) = oneshot::channel::<Result<(), Box<dyn Error + Send + Sync>>>();
        let mut find_peer_requests_lock = self.find_peer_requests.lock().await;

        if let Some(kdht) = self.swarm.behaviour_mut().kdht.as_mut() {
            let q = kdht.get_closest_peers(peer_id);
            find_peer_requests_lock.insert(q, sender);
        }

        receiver
    }
}

async fn _open_stream(mut ctrl: stream::Control, peer_id: PeerId, protocol: StreamProtocol, message: Vec<u8>) -> Result<Stream, Box<dyn Error + Send + Sync>> {
    let mut s = ctrl.open_stream(peer_id, protocol).await?;

    if !message.is_empty() {
        match s.write(&message[..1]).await {
            Ok(0) => {
                tracing::warn!("Failed to send message: check warnings");
                return Err(Box::new(io::Error::new(io::ErrorKind::ConnectionReset, "Connection reset")));
            }
            Ok(_) => {
                s.write_all(&message[1..]).await?;
            }
            Err(e) => {
                tracing::warn!("Failed to send message: {:?}", e);
                return Err(Box::new(e));
            }
        }
        s.flush().await?;
    }
    Ok(s)
}

async fn open_stream(ctrl: stream::Control, peer_id: PeerId, protocol: StreamProtocol, message: Vec<u8>, send_response: oneshot::Sender<Result<Stream, Box<dyn Error + Send + Sync>>>, find_peer_receiver: Option<oneshot::Receiver<Result<(), Box<dyn Error + Send + Sync>>>>) {
    if let Some(receiver) = find_peer_receiver {
        match receiver.await {
            Ok(Ok(_)) => {
                tracing::info!("Peer found");
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

async fn initialize_libp2p(cfg: &NetworkingConfig, receiver: mpsc::Receiver<client::Command>, event_sender: mpsc::Sender<event::Event>) -> Result<String, Box<dyn Error + Send + Sync>> {
    let res = Libp2p::new(cfg, receiver, event_sender).await;
    match res {
        Ok(node) => Ok(node.id),
        Err(e) => Err(e),
    }
}

impl Networking {
    pub fn new(cfg: &NetworkingConfig) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let (sender, receiver) = channel::<client::Command>(8);
        let (event_sender, event_receiver) = channel::<event::Event>(8);
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

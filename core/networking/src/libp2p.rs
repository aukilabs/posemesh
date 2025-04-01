use futures::{channel::{mpsc::{self, channel, Receiver}, oneshot}, lock::Mutex, AsyncWriteExt, SinkExt, StreamExt};
use libp2p::{core::muxing::StreamMuxerBox, gossipsub::{self, IdentTopic}, kad::{self, store::MemoryStore, GetClosestPeersOk, ProgressStep, QueryId}, multiaddr::{Multiaddr, Protocol}, swarm::{behaviour::toggle::Toggle, DialError, NetworkBehaviour, SwarmEvent}, PeerId, Stream, StreamProtocol, Swarm, Transport};
use std::{collections::HashMap, error::Error, io::{self, Read, Write}, str::FromStr, sync::Arc, time::Duration};
use rand::{thread_rng, rngs::OsRng};
use serde::{de, Deserialize, Serialize};
use libp2p_stream::{self as stream, IncomingStreams};
use crate::{client::{self, Client}, event};

#[cfg(not(target_family="wasm"))]
use libp2p_webrtc as webrtc;
#[cfg(not(target_family="wasm"))]
use libp2p::{mdns, noise, tcp, yamux};
#[cfg(not(target_family="wasm"))]
use tracing_subscriber::EnvFilter;
#[cfg(not(target_family="wasm"))]
use std::{fs, path::Path, net::Ipv4Addr};

#[cfg(not(target_family="wasm"))]
use tokio::spawn;
#[cfg(target_family="wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

#[cfg(target_family="wasm")]
use libp2p_webrtc_websys as webrtc_websys;
#[cfg(target_family="wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "py")]
use pyo3::prelude::*;

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
            name: "Placeholder".to_string(), // placeholder
        }
    }
}

#[cfg_attr(feature = "py", pyclass(get_all))]
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
    if !private_key_path.is_none() {
        return keypair_file(private_key_path.as_ref().unwrap());
    }

    return libp2p::identity::Keypair::generate_ed25519();
}

fn build_swarm(key: libp2p::identity::Keypair, mut behavior: PosemeshBehaviour) -> Result<Swarm<PosemeshBehaviour>, Box<dyn Error + Send + Sync>> {
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
        .with_agent_version(cfg.name.clone()),
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
    };

    #[cfg(not(target_family="wasm"))]
    if cfg.enable_mdns {
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
            .expect("Failed to build mdns behaviour");
        behavior.mdns = Some(mdns).into();
    }

    #[cfg(not(target_family="wasm"))]
    if cfg.enable_relay_server {
        let relay = libp2p::relay::Behaviour::new(key.public().to_peer_id(), Default::default());
        behavior.relay = Some(relay).into();
        behavior.autonat_server = Some(libp2p::autonat::v2::server::Behaviour::new(OsRng)).into();
    } else {
        // TODO: should not add to clients
        behavior.autonat_client = Some(libp2p::autonat::v2::client::Behaviour::new(OsRng,libp2p::autonat::v2::client::Config::default())).into();
    }

    if cfg.enable_kdht {
        let mut kad_cfg = libp2p::kad::Config::new(POSEMESH_PROTO_NAME);
        kad_cfg.set_query_timeout(Duration::from_secs(5));
        let store = libp2p::kad::store::MemoryStore::new(key.public().to_peer_id());
        let mut kdht = libp2p::kad::Behaviour::with_config(key.public().to_peer_id(), store, kad_cfg);

        #[cfg(not(target_family="wasm"))]
        kdht.set_mode(Some(kad::Mode::Server));

        #[cfg(target_family="wasm")]
        kdht.set_mode(Some(kad::Mode::Client)); // TODO: do it for all clients instead of just wasm
        
        let bootstrap_nodes = cfg.bootstrap_nodes.clone();
        for bootstrap in bootstrap_nodes {
            let peer_id = match bootstrap.split('/').last() {
                Some(peer_id) => PeerId::from_str(peer_id).unwrap(),
                None => continue,
            };
            let maddr = Multiaddr::from_str(&bootstrap).expect("Failed to parse bootstrap node address");
            let _ = kdht.add_address(&peer_id, maddr);
            // behavior.gossipsub.add_explicit_peer(&peer_id);
        }

        behavior.kdht = Some(kdht).into();
    }

    behavior
}

#[cfg(not(target_family="wasm"))]
fn build_listeners(port: u16) -> [Multiaddr; 3] {
    let mut webrtc_port = port;
    if webrtc_port != 0 {
        webrtc_port+=1;
    }
    return [
        Multiaddr::empty()
            .with(Protocol::Ip4(Ipv4Addr::UNSPECIFIED))
            .with(Protocol::Tcp(port)),
        Multiaddr::from(Ipv4Addr::UNSPECIFIED)
            .with(Protocol::Udp(webrtc_port))
            .with(Protocol::WebRTCDirect),
        Multiaddr::from(Ipv4Addr::UNSPECIFIED)
            .with(Protocol::Udp(port))
            .with(Protocol::QuicV1),
    ];
}

impl Libp2p {
    pub fn new(cfg: &NetworkingConfig, command_receiver: mpsc::Receiver<client::Command>, event_sender: mpsc::Sender<event::Event>) -> Result<Node, Box<dyn Error + Send + Sync>> {
        let private_key = cfg.private_key.clone();
        let key = parse_or_create_keypair(private_key, cfg.private_key_path.clone());
        println!("Local peer id: {:?}", key.public().to_peer_id());

        let behaviour = build_behavior(key.clone(), cfg);

        let mut swarm = build_swarm(key.clone(), behaviour)?;
        
        // let nodes_map: Arc<Mutex<HashMap<String, Node>>> = Arc::new(Mutex::new(HashMap::new()));
        // let nodes_map = HashMap::new();

        #[cfg(not(target_family="wasm"))]
        let listeners = build_listeners(cfg.port);
        #[cfg(not(target_family="wasm"))]
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

        // // Create a Gossipsub topic
        // let topic = gossipsub::IdentTopic::new("Posemesh");
        // // subscribes to our topic
        // swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

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
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                command = self.command_receiver.select_next_some() => self.handle_command(command).await,
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
                for peer in peers {
                    self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer.peer_id);
                    for addr in peer.addrs {
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
                    let _ = sender.unwrap().send(Ok(()));
                    return;
                } else if last {
                    tracing::error!("Failed to find peer: {peer_id}");
                }
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
                peer_id, ..
            } => {
                tracing::info!("Connected to {peer_id}");
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
                tracing::info!("Received message from {:?} on topic {topic}", source);
                if let Err(e) = self.event_sender.send(event::Event::PubSubMessageReceivedEvent { 
                        topic: topic.clone(),
                        message: data.clone(),
                        from: source.clone(),
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
            }
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::AutonatClient(libp2p::autonat::v2::client::Event {
                server,
                tested_addr,
                bytes_sent,
                result: Err(e),
            })) => {
                tracing::info!("Tested {tested_addr} with {server}. Sent {bytes_sent} bytes for verification. Failed with {e:?}.");
                // TODO: should be done only once and not for every failed autonat test
                // client should not care
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
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::AutonatServer(libp2p::autonat::v2::server::Event {tested_addr, ..})) => {
                tracing::info!("Autonat Server tested address: {tested_addr}");
            }
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Identify(libp2p::identify::Event::Received {
                info: libp2p::identify::Info { observed_addr, listen_addrs, protocols, agent_version, .. },
                peer_id,
                ..
            })) =>
            {
                tracing::info!("Observed address: {observed_addr} for {peer_id}");
                if self.cfg.enable_relay_server {
                    self.swarm.add_external_address(observed_addr.clone());
                }

                // TODO: Only add the non local address to the DHT
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

                // self.nodes_map.insert(node.id.clone(), node.clone());
                self.event_sender.send(event::Event::NewNodeRegistered { node: node.clone() }).await.unwrap();
                
                // #[cfg(target_family="wasm")]
                // self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            },
            e => tracing::debug!("Other events: {e:?}"),
        }
    }

    async fn handle_command(&mut self, command: client::Command) {
        match command {
            client::Command::Send { message, peer_id, protocol, response } => {
                let ctrl = self.swarm.behaviour_mut().streams.new_control();
                let (sender, mut receiver) = oneshot::channel::<Result<(), Box<dyn Error + Send + Sync>>>();

                if !Swarm::is_connected(&self.swarm, &peer_id) {
                    self.find_peer(peer_id, sender).await;
                } else {
                    receiver.close();
                }
                #[cfg(target_family="wasm")]
                wasm_bindgen_futures::spawn_local(stream(ctrl, peer_id, protocol, message, response, receiver));

                #[cfg(not(target_family="wasm"))]
                tokio::spawn(stream(ctrl, peer_id, protocol, message, response, receiver));
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
            Ok(_) => {
                return;
            },
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

    async fn find_peer(&mut self, peer_id: PeerId, sender: oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>) {
        let mut find_peer_requests_lock = self.find_peer_requests.lock().await;
        // TODO: add timeout to the query
        self.swarm.behaviour_mut().kdht.as_mut().map(|dht| {
            let q = dht.get_closest_peers(peer_id.clone());
            find_peer_requests_lock.insert(q, sender);
        });
    }
}

async fn stream(mut ctrl: stream::Control, peer_id: PeerId, protocol: StreamProtocol, message: Vec<u8>, send_response: oneshot::Sender<Result<Stream, Box<dyn Error + Send + Sync>>>, find_response: oneshot::Receiver<Result<(), Box<dyn Error + Send + Sync>>>) {
    if let Ok(Err(e)) = find_response.await {
        tracing::error!("{}", e);
    }

    let mut stream = match ctrl.open_stream(peer_id, protocol).await {
        Ok(stream) => stream,
        Err(error @ stream::OpenStreamError::UnsupportedProtocol(_)) => {
            if let Err(send_err) = send_response.send(Err(Box::new(error))) {
                tracing::error!("Failed to send feedback: {:?}", send_err);
            }
            return;
        }
        Err(error) => {
            if let Err(send_err) = send_response.send(Err(Box::new(error))) {
                tracing::error!("Failed to send feedback: {:?}", send_err);
            }
            return;
        }
    };
    if message.len() != 0 {
        if let Err(e) = stream.write_all(&message).await {
            if let Err(send_err) = send_response.send(Err(Box::new(e))) {
                tracing::error!("Failed to send feedback: {:?}", send_err);
            }
            return;
        }
        if let Err(e) = stream.flush().await {
            if let Err(send_err) = send_response.send(Err(Box::new(e))) {
                tracing::error!("Failed to send feedback: {:?}", send_err);
            }
            return;
        }
    }
    
    if let Err(send_err) = send_response.send(Ok(stream)) {
        tracing::error!("Failed to send feedback: {:?}", send_err);
    }
}


#[derive(Clone)]
pub struct Networking {
    pub client: Client,
    pub event_receiver: Arc<Mutex<Receiver<event::Event>>>,
    pub id: String,
}

impl Networking {
    pub fn new(cfg: &NetworkingConfig) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let (sender, receiver) = channel::<client::Command>(8);
        let (event_sender, event_receiver) = channel::<event::Event>(8);
        let cfg = cfg.clone();
        let client = Client::new(sender);
        
        let node = Libp2p::new(&cfg, receiver, event_sender)?;

        Ok(Networking {
            client,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            id: node.id,
        })
    }
}

use futures::{AsyncReadExt, AsyncWriteExt, StreamExt, SinkExt};
use libp2p::{core::muxing::StreamMuxerBox, gossipsub::{self, IdentTopic}, kad::{self, store::MemoryStore}, multiaddr::{Multiaddr, Protocol}, swarm::{behaviour::toggle::Toggle, NetworkBehaviour, SwarmEvent}, PeerId, Stream, StreamProtocol, Swarm, Transport};
use std::{collections::HashMap, str::FromStr};
use std::error::Error;
use std::time::Duration;
use rand::{thread_rng, rngs::OsRng};
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::{Arc, Mutex};
use libp2p_stream::{self as stream, IncomingStreams};
use std::io::{self, Read, Write};
use crate::{client, event};

#[cfg(not(target_family="wasm"))]
use libp2p_webrtc as webrtc;
#[cfg(not(target_family="wasm"))]
use libp2p::{mdns, noise, tcp, yamux};
#[cfg(not(target_family="wasm"))]
use tracing_subscriber::EnvFilter;
#[cfg(not(target_family="wasm"))]
use std::{fs, path::Path, net::Ipv4Addr};
#[cfg(not(target_family="wasm"))]
use tokio::time::interval;

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
    pub private_key: String,
    pub private_key_path: String,
    pub enable_kdht: bool,
    pub name: String,
    pub node_types: Vec<String>,
    pub node_capabilities: Vec<String>,
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
            private_key: "".to_string(),
            private_key_path: "./volume/pkey".to_string(),
            name: "c++ server".to_string(), // placeholder
            node_capabilities: vec![], // placeholder
            node_types: vec!["c++ server".to_string()], // placeholder
        }
    }
}

#[cfg_attr(feature = "py", pyclass(get_all))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Node {
    pub id: String,
    pub name: String,
    pub node_types: Vec<String>,   // Assuming node_types is a list of strings
    pub capabilities: Vec<String>, // Assuming capabilities is a list of strings
}

const CHAT_PROTOCOL: StreamProtocol = StreamProtocol::new("/chat");
const POSEMESH_PROTO_NAME: StreamProtocol = StreamProtocol::new("/posemesh/kad/1.0.0");

pub struct Networking {
    nodes_map: Arc<Mutex<HashMap<String, Node>>>,
    swarm: Swarm<PosemeshBehaviour>,
    cfg: NetworkingConfig,
    command_receiver: futures::channel::mpsc::Receiver<client::Command>,
    node: Node,
    node_regsiter_topic: IdentTopic,
    event_sender: futures::channel::mpsc::Sender<event::Event>,
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
    private_key: &mut [u8],
    private_key_path: &String,
) -> libp2p::identity::Keypair {
    // load private key into keypair
    if let Ok(keypair) = libp2p::identity::Keypair::ed25519_from_bytes(private_key) {
        return keypair;
    }

    #[cfg(not(target_family="wasm"))]
    return keypair_file(private_key_path);

    #[cfg(target_family="wasm")]
    return libp2p::identity::Keypair::generate_ed25519();
}

fn build_swarm(key: libp2p::identity::Keypair, mut behavior: PosemeshBehaviour) -> Result<Swarm<PosemeshBehaviour>, Box<dyn Error>> {
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
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(key)
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
        libp2p::identify::Config::new("/posemesh/id/1.0.0".to_string(), key.public()),
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

impl Networking {
    pub(crate) fn new(cfg: &NetworkingConfig, command_receiver: futures::channel::mpsc::Receiver<client::Command>, event_sender: futures::channel::mpsc::Sender<event::Event>) -> Result<Self, Box<dyn Error>> {
        #[cfg(not(target_family="wasm"))]
        let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();
        
        #[cfg(target_family="wasm")]
        tracing_wasm::set_as_global_default();

        let mut private_key = cfg.private_key.clone();
        let private_key_bytes = unsafe {private_key.as_bytes_mut()};
        let key = parse_or_create_keypair(private_key_bytes, &cfg.private_key_path);
        println!("Local peer id: {:?}", key.public().to_peer_id());

        let behaviour = build_behavior(key.clone(), cfg);

        let mut swarm = build_swarm(key.clone(), behaviour)?;

        let bootstrap_nodes = cfg.bootstrap_nodes.clone();
        for bootstrap in bootstrap_nodes {
            let peer_id = match bootstrap.split('/').last() {
                Some(peer_id) => PeerId::from_str(peer_id).unwrap(),
                None => continue,
            };
            let maddr = Multiaddr::from_str(&bootstrap)?;
            tracing::info!("Adding peer to DHT: {:?}", peer_id);

            swarm
                .behaviour_mut()
                .kdht
                .as_mut()
                .map(|dht| { dht.add_address(&peer_id, maddr.clone())});

            swarm.dial(maddr)?;
        }
        
        let nodes_map: Arc<Mutex<HashMap<String, Node>>> = Arc::new(Mutex::new(HashMap::new()));

        #[cfg(not(target_family="wasm"))]
        let listeners = build_listeners(cfg.port);
        #[cfg(not(target_family="wasm"))]
        for addr in listeners.iter() {
            swarm.listen_on(addr.clone())?;
        }

        let node = Node{
            id: key.public().to_peer_id().to_string(),
            name: cfg.name.clone(),
            node_types: cfg.node_types.clone(),
            capabilities: cfg.node_capabilities.clone(),
        };

        // Create a Gossipsub topic
        let topic = gossipsub::IdentTopic::new("Posemesh");
        // subscribes to our topic
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

        Ok(Networking {
            cfg: cfg.clone(),
            nodes_map: nodes_map,
            swarm: swarm,
            command_receiver: command_receiver,
            node: node,
            node_regsiter_topic: topic,
            event_sender: event_sender,
        })
    }

    pub async fn run(mut self) -> io::Result<()> {
        tracing::info!("Starting networking");
        
        #[cfg(not(target_family="wasm"))]
        let mut node_register_interval = interval(Duration::from_secs(10));

        let chat_stream = self.swarm.behaviour_mut().streams.new_control().accept(CHAT_PROTOCOL).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        #[cfg(target_family="wasm")]
        wasm_bindgen_futures::spawn_local(chat_protocol_handler(chat_stream, self.event_sender.clone()));

        #[cfg(not(target_family="wasm"))]
        tokio::spawn(chat_protocol_handler(chat_stream, self.event_sender.clone()));

        #[cfg(not(target_family="wasm"))]
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                command = self.command_receiver.select_next_some() => self.handle_command(command).await,
                _ = node_register_interval.tick() => {
                    match self.register_node() {
                        Ok(_) => {},
                        Err(e) => {
                            tracing::warn!("Failed to register node: {e}");
                        }
                    }
                }
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
                    result: kad::QueryResult::GetClosestPeers(Ok(ok)),
                ..
                },
            )) => {
                if ok.peers.is_empty() {
                    tracing::info!("Query finished with no closest peers");
                    return;
                }
                tracing::info!("Query finished with closest peers: {:#?}", ok.peers);
                ok.peers.iter().for_each(|peer| {
                    self.swarm.behaviour_mut().kdht.as_mut().map(|dht| {
                        peer.addrs.iter().for_each(|addr| {
                            dht.add_address(&peer.peer_id, addr.clone());
                        });
                    });
                });
            },
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
                propagation_source: _peer_id,
                message_id: _id,
                message,
            })) => {
                #[cfg(target_family="wasm")]
                match self.register_node() {
                    Ok(_) => {},
                    Err(e) => {
                        tracing::error!("Failed to register node: {e}");
                    }
                }
                match serde_json::from_slice::<Node>(&message.data) {
                    Ok(node) => {
                        if self.nodes_map.lock().unwrap().contains_key(&node.id) {
                            return;
                        }
                        if node.id == *self.swarm.local_peer_id().to_string() {
                            return;
                        }
                        println!("Node {} joins the network", node.name);
                        self.nodes_map.lock().unwrap().insert(node.id.clone(), node.clone());
                        self.event_sender.send(event::Event::NewNodeRegistered { node: node.clone() }).await.unwrap();
                    },
                    Err(e) => {
                        tracing::info!("Failed to deserialize node info: {}", e);
                    }
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
            SwarmEvent::Behaviour(event) => {
                tracing::info!("{event:?}");
                if let PosemeshBehaviourEvent::Identify(libp2p::identify::Event::Received {
                    info: libp2p::identify::Info { observed_addr, listen_addrs, .. },
                    peer_id,
                    ..
                }) = &event
                {
                    tracing::info!("Observed address: {observed_addr} for {peer_id}");
                    if self.cfg.enable_relay_server {
                        self.swarm.add_external_address(observed_addr.clone());
                    }

                    for addr in listen_addrs {
                        self.swarm.behaviour_mut().kdht.as_mut().map(|dht| {
                            dht.add_address(peer_id, addr.clone());
                        });
                    }

                    #[cfg(target_family="wasm")]
                    self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                }
            },
            e => tracing::debug!("{e:?}"),
        }
    }

    async fn handle_command(&mut self, command: client::Command) {
        match command {
            client::Command::Send { message, peer_id, protocol } => {
                let mut ctrl = self.swarm.behaviour_mut().streams.new_control();
                // TODO: find node in DHT and dial
                // self.swarm.behaviour_mut().kdht.as_mut().map(|dht| {
                //     if let Some(r) = dht.kbucket(peer_id) {
                //         self.swarm.dial(r.addrs[0].clone()).unwrap();
                //     }
                // });
                let stream = match ctrl.open_stream(peer_id, protocol).await {
                    Ok(stream) => stream,
                    Err(error @ stream::OpenStreamError::UnsupportedProtocol(_)) => {
                        tracing::info!(%peer_id, %error);
                        return;
                    }
                    Err(error) => {
                        // Other errors may be temporary.
                        // In production, something like an exponential backoff / circuit-breaker may be more appropriate.
                        tracing::debug!(%peer_id, %error);
                        return;
                    }
                };

                if let Err(e) = send(stream, message).await {
                    tracing::warn!(%peer_id, "Chat protocol failed: {e}");
                    return;
                }
            },

        }
    }

    fn register_node(self: &mut Self) -> Result<(), Box<dyn Error>> {
        let serialized = serde_json::to_vec(&self.node.clone())?;
        self.swarm.behaviour_mut().gossipsub.publish(self.node_regsiter_topic.clone(), serialized)?;
        Ok(())
    }
}

async fn _send_message(mut controller: stream::Control, peer: PeerId, msg: Vec<u8>) {
    let stream = match controller.open_stream(peer, CHAT_PROTOCOL).await {
        Ok(stream) => stream,
        Err(error @ stream::OpenStreamError::UnsupportedProtocol(_)) => {
            tracing::error!(%peer, %error);
            return;
        }
        Err(error) => {
            // Other errors may be temporary.
            // In production, something like an exponential backoff / circuit-breaker may be more appropriate.
            tracing::debug!(%peer, %error);
            return;
        }
    };

    if let Err(e) = send(stream, msg).await {
        tracing::warn!(%peer, "Chat protocol failed: {e}");
        return;
    }
}

async fn send(mut stream: Stream, msg: Vec<u8>) -> io::Result<()> {
    stream.write_all(&msg).await?;
    stream.close().await?;

    Ok(())
}

async fn _receive_message(mut stream: Stream) -> io::Result<Vec<u8>> {
    let mut buf = [0u8; 100];

    loop {
        let read = stream.read(&mut buf).await?;
        if read == 0 {
            return Ok("EOF".as_bytes().to_vec());
        }

        tracing::info!("Received: {:?}", std::str::from_utf8(&buf[..read]).unwrap()); 
        return Ok(buf[..read].to_vec());
    }
}

async fn chat_protocol_handler(mut incoming_stream: IncomingStreams, mut event_sender: futures::channel::mpsc::Sender<event::Event>) {
    while let Some((peer, stream)) = incoming_stream.next().await {
        let _ = event_sender
            .send(event::Event::MessageReceived { 
                stream,
                protocol: CHAT_PROTOCOL,
                peer,
             })
            .await; 
        // match _receive_message(stream).await {
        //     Ok(n) => {
        //         let _ = event_sender
        //             .send(event::Event::MessageReceived { message: n })
        //             .await;
        //     }
        //     Err(e) => {
        //         tracing::warn!(%peer, "Receive failed: {e}");
        //         continue;
        //     }
        // };
    }
}

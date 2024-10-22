use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use libp2p::{core::muxing::StreamMuxerBox, gossipsub, kad::{self, store::MemoryStore}, multiaddr::{Multiaddr, Protocol}, swarm::{behaviour::toggle::Toggle, NetworkBehaviour, SwarmEvent}, PeerId, Stream, StreamProtocol, Swarm, Transport};
use std::{collections::HashMap ,str::FromStr};
use std::error::Error;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::{Arc, Mutex};
use libp2p_stream as stream;
use std::io::{self, Read, Write};

#[cfg(not(target_arch = "wasm32"))]
use tokio::{select, time::interval};
#[cfg(not(target_arch = "wasm32"))]
use libp2p_webrtc as webrtc;
#[cfg(not(target_arch = "wasm32"))]
use rand::thread_rng;
#[cfg(not(target_arch = "wasm32"))]
use libp2p::{mdns, noise, tcp, yamux};
#[cfg(not(target_arch = "wasm32"))]
use tracing_subscriber::EnvFilter;
#[cfg(not(target_arch = "wasm32"))]
use std::{fs, path::Path, net::Ipv4Addr};

#[cfg(target_arch = "wasm32")]
use libp2p_webrtc_websys as webrtc_websys;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
struct PosemeshBehaviour {
    gossipsub: gossipsub::Behaviour,
    streams: stream::Behaviour,
    identify: libp2p::identify::Behaviour,
    kdht: Toggle<libp2p::kad::Behaviour<MemoryStore>>,
    #[cfg(not(target_arch = "wasm32"))]
    mdns: Toggle<mdns::tokio::Behaviour>,
    #[cfg(not(target_arch = "wasm32"))]
    relay: Toggle<libp2p::relay::Behaviour>,
}

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    id: PeerId,
    name: String,
    node_types: Vec<String>,   // Assuming node_types is a list of strings
    capabilities: Vec<String>, // Assuming capabilities is a list of strings
}

const CHAT_PROTOCOL: StreamProtocol = StreamProtocol::new("/chat");
const POSEMESH_PROTO_NAME: StreamProtocol = StreamProtocol::new("/posemesh/kad/1.0.0");


pub(crate) struct RNetworking {
    pub nodes_map: Arc<Mutex<HashMap<PeerId, Node>>>,
    messages: Arc<Mutex<Vec<Vec<u8>>>>,
    incoming_streams: stream::Control,
}

#[cfg(not(target_arch = "wasm32"))]
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
            eprintln!("Failed to create directory: {err}");
        }
    }

    // Save the new keypair to the file
    if let Ok(mut file) = fs::File::create(path) {
        let keypair_bytes = keypair.to_protobuf_encoding().expect("Failed to encode keypair");
        if file.write_all(&keypair_bytes).is_err() {
            eprintln!("Failed to write keypair to file");
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

    #[cfg(not(target_arch = "wasm32"))]
    return keypair_file(private_key_path);

    #[cfg(target_arch = "wasm32")]
    return libp2p::identity::Keypair::generate_ed25519();
}

async fn handle_events(mut swarm: Swarm<PosemeshBehaviour>, nodes_map: Arc<Mutex<HashMap<PeerId, Node>>>, node: Node) {
    #[cfg(not(target_arch = "wasm32"))]
    let mut publish_check_interval = interval(Duration::from_secs(10));
    #[cfg(not(target_arch = "wasm32"))]
    let mut connected_peers = 0;

    // Create a Gossipsub topic
    let topic = gossipsub::IdentTopic::new("Posemesh");
    // subscribes to our topic
    if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&topic) {
        eprintln!("Failed to subscribe to topic: {e}");
        return;
    }

    #[cfg(not(target_arch = "wasm32"))]
    loop {
        select! {
            // Publish node info when there are discovered peers
            _ = publish_check_interval.tick() => {
                if connected_peers >= 1 {  // Set your own threshold
                    match serde_json::to_vec(&node) {
                        Ok(serialized) => {
                            match swarm.behaviour_mut().gossipsub.publish(topic.clone(), serialized) {
                                Ok(_) => {
                                    // println!("Successfully published node info");
                                    // published = true;
                                }
                                Err(e) => {
                                    // Handle publish error
                                    eprintln!("Failed to publish node info: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to serialize node info: {}", e);
                        }
                    }
                }
            },
            event = swarm.select_next_some() => match event  {
                SwarmEvent::Behaviour(PosemeshBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discovered a new peer: {peer_id}");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        connected_peers+=1;
                    }
                },
                SwarmEvent::Behaviour(PosemeshBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discover peer has expired: {peer_id}");
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                        connected_peers-=1;
                    }
                },
                SwarmEvent::Behaviour(PosemeshBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: _peer_id,
                    message_id: _id,
                    message,
                })) => {
                    match serde_json::from_slice::<Node>(&message.data) {
                        Ok(node) => {
                            if nodes_map.lock().unwrap().contains_key(&node.id) {
                                continue;
                            }
                            if node.id == *swarm.local_peer_id() {
                                continue;
                            }
                            println!("Node {} joins the network", node.name);
                            nodes_map.lock().unwrap().insert(node.id.clone(), node);
                        },
                        Err(e) => {
                            println!("Failed to deserialize node info: {}", e);
                        }
                    }
                },
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Local node is listening on {address}");
                },
                // Prints peer id identify info is being sent to.
                SwarmEvent::Behaviour(PosemeshBehaviourEvent::Identify(libp2p::identify::Event::Sent { peer_id, .. })) => {
                    println!("Sent identify info to {peer_id:?}")
                }
                SwarmEvent::Behaviour(PosemeshBehaviourEvent::Kdht(kad::Event::OutboundQueryProgressed {
                    result: kad::QueryResult::GetClosestPeers(Ok(ok)),
                    ..
                })) => {
                    if ok.peers.is_empty() {
                        println!("Query finished with no closest peers");
                    } else {
                        println!("Query finished with closest peers: {:#?}", ok.peers);
                    }
                    println!("Query finished with closest peers: {:#?}", ok.peers);
                }
                SwarmEvent::Behaviour(event) => {
                    if let PosemeshBehaviourEvent::Identify(libp2p::identify::Event::Received {
                        info: libp2p::identify::Info { observed_addr, .. },
                        ..
                    }) = &event
                    {
                        swarm.add_external_address(observed_addr.clone());
                        println!("{event:?}")
                    }
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    connected_peers+=1; 
                }
                _ => {}
            }
        }
    }

    #[cfg(target_arch= "wasm32")]
    loop {
        match swarm.next().await.unwrap() {
            SwarmEvent::Behaviour(PosemeshBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source: _peer_id,
                message_id: _id,
                message,
            })) => {
                match serde_json::from_slice::<Node>(&message.data) {
                    Ok(new_node) => {
                        match serde_json::to_vec(&node) {
                            Ok(serialized) => {
                                match swarm.behaviour_mut().gossipsub.publish(topic.clone(), serialized) {
                                    Ok(_) => {
                                        tracing::debug!("Successfully published node info");
                                        // published = true;
                                    }
                                    Err(e) => {
                                        // Handle publish error
                                        tracing::debug!("Failed to publish node info: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to serialize node info: {}", e);
                            }
                        }
                        if nodes_map.lock().unwrap().contains_key(&new_node.id) {
                            continue;
                        }
                        tracing::debug!("Node {} joins the network", new_node.name);
                        nodes_map.lock().unwrap().insert(new_node.id.clone(), new_node);
                    },
                    Err(e) => {
                        println!("Failed to deserialize node info: {}", e);
                    }
                }
            },
            SwarmEvent::Behaviour(event) => {
                if let PosemeshBehaviourEvent::Identify(libp2p::identify::Event::Received {
                    info: libp2p::identify::Info { observed_addr, .. },
                    peer_id,
                    ..
                }) = &event
                {
                    swarm.add_external_address(observed_addr.clone());
                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                }
                tracing::debug!("Accepted event: {event:?}")
            }
            _ => {}
        }
    }
}

fn build_swarm(key: libp2p::identity::Keypair, behavior: PosemeshBehaviour) -> Result<Swarm<PosemeshBehaviour>, Box<dyn Error>> {
    #[cfg(not(target_arch = "wasm32"))]
    let swarm = libp2p::SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
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
        .with_behaviour(|_| behavior)?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    #[cfg(target_arch = "wasm32")]
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
        kdht: None.into(),
        #[cfg(not(target_arch = "wasm32"))]
        mdns: None.into(),
        #[cfg(not(target_arch = "wasm32"))]
        relay: None.into(),
    };

    #[cfg(not(target_arch = "wasm32"))]
    if cfg.enable_mdns {
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
            .expect("Failed to build mdns behaviour");
        behavior.mdns = Some(mdns).into();
    }

    #[cfg(not(target_arch = "wasm32"))]
    if cfg.enable_relay_server {
        let relay = libp2p::relay::Behaviour::new(key.public().to_peer_id(), Default::default());
        behavior.relay = Some(relay).into();
    }

    if cfg.enable_kdht {
        let mut kad_cfg = libp2p::kad::Config::new(POSEMESH_PROTO_NAME);
        kad_cfg.set_query_timeout(Duration::from_secs(5 * 60));
        let store = libp2p::kad::store::MemoryStore::new(key.public().to_peer_id());
        let kdht = libp2p::kad::Behaviour::with_config(key.public().to_peer_id(), store, kad_cfg);
        behavior.kdht = Some(kdht).into();
    }

    behavior
}

#[cfg(not(target_arch = "wasm32"))]
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

impl RNetworking {
    pub fn new(cfg: &NetworkingConfig) -> Result<Self, Box<dyn Error>> {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();
        
        #[cfg(target_arch = "wasm32")]
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
                .map(|dht| {dht.add_address(&peer_id, maddr.clone())});

            swarm.dial(maddr)?;
        }

        let mut incoming_streams = swarm
            .behaviour_mut()
            .streams
            .new_control()
            .accept(CHAT_PROTOCOL)
            .unwrap();
        let stream_control = swarm
            .behaviour_mut()
            .streams.new_control();
        
        let nodes_map: Arc<Mutex<HashMap<PeerId, Node>>> = Arc::new(Mutex::new(HashMap::new()));
        let nodes_map_clone = nodes_map.clone();

        let messages: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
        let messages_clone = messages.clone();

        #[cfg(not(target_arch = "wasm32"))]
        let listeners = build_listeners(cfg.port);
        #[cfg(not(target_arch = "wasm32"))]
        for addr in listeners.iter() {
            swarm.listen_on(addr.clone())?;
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        tokio::spawn(async move {
            // This loop handles incoming streams _sequentially_ but that doesn't have to be the case.
            // You can also spawn a dedicated task per stream if you want to.
            // Be aware that this breaks backpressure though as spawning new tasks is equivalent to an unbounded buffer.
            // Each task needs memory meaning an aggressive remote peer may force you OOM this way.
            
            while let Some((peer, stream)) = incoming_streams.next().await {
                match _receive_message(stream, messages_clone.clone()).await {
                    Ok(n) => {
                        tracing::info!(%peer, "Echoed {n} bytes!");
                    }
                    Err(e) => {
                        tracing::warn!(%peer, "Echo failed: {e}");
                        continue;
                    }
                };
            }
        });

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            // This loop handles incoming streams _sequentially_ but that doesn't have to be the case.
            // You can also spawn a dedicated task per stream if you want to.
            // Be aware that this breaks backpressure though as spawning new tasks is equivalent to an unbounded buffer.
            // Each task needs memory meaning an aggressive remote peer may force you OOM this way.
            
            while let Some((peer, stream)) = incoming_streams.next().await {
                match _receive_message(stream, messages_clone.clone()).await {
                    Ok(n) => {
                        tracing::info!(%peer, "Echoed {n} bytes!");
                    }
                    Err(e) => {
                        tracing::warn!(%peer, "Echo failed: {e}");
                        continue;
                    }
                };
            }
        });

        let node = Node{
            id: key.public().to_peer_id(),
            name: cfg.name.clone(),
            node_types: cfg.node_types.clone(),
            capabilities: cfg.node_capabilities.clone(),
        };

        #[cfg(not(target_arch = "wasm32"))]
        tokio::spawn(handle_events(swarm, nodes_map_clone, node));

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(handle_events(swarm, nodes_map_clone, node));

        Ok(RNetworking {
            nodes_map: nodes_map,
            messages: messages,
            incoming_streams: stream_control,
        })
    }

    pub fn send_message(&mut self, msg: Vec<u8>) {
        let nodes = self.nodes_map.lock().unwrap();
        nodes.iter().for_each(|(peer, _)| {
            let peer_clone = peer.clone();
            let msg_clone = msg.clone();
            let incoming_streams = self.incoming_streams.clone();
            println!("Sending message to peer: {:?}", peer);
            #[cfg(not(target_arch = "wasm32"))]
            tokio::spawn(_send_message(incoming_streams, peer_clone, msg_clone));
            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(_send_message(incoming_streams, peer_clone, msg_clone));
        });
    }

    pub fn poll_messages(&mut self) -> Vec<Vec<u8>> {
        let mut messages = self.messages.lock().unwrap();
        let messages_clone = messages.clone();
        messages.clear();
        messages_clone
    }
}

async fn _send_message(mut controller: stream::Control, peer: PeerId, msg: Vec<u8>) {
    let stream = match controller.open_stream(peer, CHAT_PROTOCOL).await {
        Ok(stream) => stream,
        Err(error @ stream::OpenStreamError::UnsupportedProtocol(_)) => {
            tracing::info!(%peer, %error);
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

async fn _receive_message(mut stream: Stream, messages: Arc<Mutex<Vec<Vec<u8>>>>) -> io::Result<usize> {
    let mut total = 0;

    let mut buf = [0u8; 100];

    loop {
        let read = stream.read(&mut buf).await?;
        if read == 0 {
            return Ok(total);
        }

        total += read;
        messages.lock().unwrap().push(buf[..read].to_vec());
        // print the message as string
        println!("Received: {:?}", std::str::from_utf8(&buf[..read]).unwrap()); 
    }
}

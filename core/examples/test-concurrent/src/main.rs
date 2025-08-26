use std::time::Duration;

use futures::{AsyncReadExt, StreamExt, AsyncWriteExt};
use posemesh_networking::{libp2p::{Networking, NetworkingConfig}, client::TClient};
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let networking = NetworkingConfig {
        port: 8080,
        bootstrap_nodes: vec![],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: None,
        private_key_path: Some("./volume/test-concurrent/bootstrap/pkey".to_string()),
        enable_websocket: false,
        enable_webrtc: false,
        namespace: None,
    };

    let protocol = "/chat/v1";
    let mut bootstrap = Networking::new(&networking).unwrap();
    let mut chat_protocol = bootstrap.client.set_stream_handler(protocol).await.unwrap();

    let bootstrap_id = bootstrap.id;
    let bootstrap_id_clone = bootstrap_id.clone();
    let bootstrap_id_clone_clone = bootstrap_id_clone.clone();
    let bootstrap_id_clone_clone_clone = bootstrap_id_clone_clone.clone();
    tokio::spawn(async move {
        while let Some((peer, mut stream)) = chat_protocol.next().await {
            let buf = &mut Vec::new();
            let _ = stream.read_to_end(buf).await.unwrap_or_else(|_| panic!("can't read from stream {}", peer));

            let msg = String::from_utf8_lossy(buf);
            println!("Bootstrap: Received message: {}", msg);
            // sleep(Duration::from_millis(800)).await;

            stream.write_all(buf).await.unwrap_or_else(|_| panic!("can't write from stream {}", peer));
            stream.flush().await.expect("can't flush stream");
        }
        eprintln!("Bootstrap: Stream handler closed");
    });

    let peer_a_cfg = NetworkingConfig {
        port: 8082,
        bootstrap_nodes: vec![format!("/ip4/127.0.0.1/udp/8080/quic-v1/p2p/{}", bootstrap_id.clone())],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: None,
        private_key_path: Some("./volume/test-concurrent/peer-a/pkey".to_string()),
        enable_websocket: false,
        enable_webrtc: false,
        namespace: None,
    };
    let mut peer_a = Networking::new(&peer_a_cfg).unwrap();
    let _peer_clone = peer_a.clone();

    let peer_b_cfg = NetworkingConfig {
        port: 8084,
        bootstrap_nodes: vec![format!("/ip4/127.0.0.1/udp/8080/quic-v1/p2p/{}", bootstrap_id)],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: None,
        private_key_path: Some("./volume/test-concurrent/peer-b/pkey".to_string()),
        enable_websocket: false,
        enable_webrtc: false,
        namespace: None,
    };
    let mut peer_b = Networking::new(&peer_b_cfg).unwrap();

    let peer_c_cfg = NetworkingConfig {
        port: 8086,
        bootstrap_nodes: vec![format!("/ip4/127.0.0.1/udp/8080/quic-v1/p2p/{}", bootstrap_id)],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: None,
        private_key_path: Some("./volume/test-concurrent/peer-c/pkey".to_string()),
        enable_websocket: false,
        enable_webrtc: false,
        namespace: None,
    };
    let mut peer_c = Networking::new(&peer_c_cfg).unwrap();

    sleep(Duration::from_millis(3000)).await;
    tokio::spawn(async move {
        // sleep(Duration::from_millis(500)).await;
        println!("{}: Sending message", peer_b.id);
        let mut s = peer_b.client.send(format!("3 - send from {}", peer_b.id).as_bytes().to_vec(), &bootstrap_id_clone, protocol, 0).await.unwrap_or_else(|_| panic!("{}: can't send message", peer_b.id));
        s.flush().await.expect("can't flush stream");
        s.close().await.expect("can't close stream");
        let buf = &mut Vec::new();
        let _ = s.read_to_end(buf).await.expect("can't read from stream");
        
        let msg = String::from_utf8_lossy(buf);
        println!("{}: Received message: {}", peer_b.id, msg);
    });
    tokio::spawn(async move {
        println!("{}: Sending message", peer_c.id.clone());
        let mut s = peer_c.client.send(format!("1 - send from {}", peer_c.id).as_bytes().to_vec(), &bootstrap_id_clone_clone, protocol, 0).await.unwrap_or_else(|_| panic!("{}: can't send message", peer_c.id));
        s.flush().await.expect("can't flush stream");
        s.close().await.expect("can't close stream");
        let buf = &mut Vec::new();
        let _ = s.read_to_end(buf).await.expect("can't read from stream");
        
        let msg = String::from_utf8_lossy(buf);
        println!("{}: Received message: {}", peer_c.id, msg);
    });

    tokio::spawn(async move {
        println!("{}: Sending message", peer_a.id);
        let mut s = peer_a.client.send(format!("2 - send from {}", peer_a.id).as_bytes().to_vec(), &bootstrap_id_clone_clone_clone, protocol, 0).await.unwrap_or_else(|_| panic!("{}: can't send message", peer_a.id));
        s.flush().await.expect("can't flush stream");
        s.close().await.expect("can't close stream");
        
        let buf = &mut Vec::new();
        let _ = s.read_to_end(buf).await.expect("can't read from stream");
        
        let msg = String::from_utf8_lossy(buf);
        println!("{}: Received message: {}", peer_a.id, msg);
    });
    
    // tokio::spawn(async move {
    //     let mut s = peer_clone.client.send(format!("2 - send from {}", peer_clone.id).as_bytes().to_vec(), bootstrap_id_clone.clone(), protocol_clone.clone(), 0).await.expect(&format!("{}: can't send message", peer_clone.id));
    //     s.flush().await.expect("can't flush stream");
    //     s.close().await.expect("can't close stream");
        
    //     let buf = &mut Vec::new();
    //     let _ = s.read_to_end(buf).await.expect("can't read from stream");
        
    //     let msg = String::from_utf8_lossy(buf);
    //     println!("{}: Received message: {}", peer_clone.id, msg);
    // }); 

    loop {

    }
}

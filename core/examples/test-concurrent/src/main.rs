use std::time::Duration;

use futures::{AsyncReadExt, StreamExt, AsyncWriteExt};
use networking::network::NetworkingConfig;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let _ = tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let networking = NetworkingConfig {
        port: 8080,
        bootstrap_nodes: vec![],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: vec![],
        private_key_path: "./volume/test-concurrent/bootstrap/pkey".to_string(),
        name: "test-concurrent/bootstrap".to_string(),
    };

    let protocol = "/chat/v1".to_string();
    let protocol_clone = protocol.clone();
    let protocol_clone_clone = protocol.clone();
    let protocol_clone_clone_clone = protocol.clone();

    let mut bootstrap = networking::context::context_create(&networking).unwrap();
    let mut chat_protocol = bootstrap.set_stream_handler(protocol.clone()).await.unwrap();

    let bootstrap_id = bootstrap.id.clone();
    let bootstrap_id_clone = bootstrap_id.clone();
    let bootstrap_id_clone_clone = bootstrap_id.clone();
    let bootstrap_id_clone_clone_clone = bootstrap_id.clone();
    tokio::spawn(async move {
        while let Some((peer, mut stream)) = chat_protocol.next().await {
            let buf = &mut Vec::new();
            let _ = stream.read_to_end(buf).await.expect(&format!("can't read from stream {}", peer));

            let msg = String::from_utf8_lossy(buf);
            println!("Bootstrap: Received message: {}", msg);
            sleep(Duration::from_millis(10)).await;

            let _ = stream.write_all(buf).await.expect(&format!("can't write from stream {}", peer));
            stream.flush().await.expect("can't flush stream");
        }
        eprintln!("Bootstrap: Stream handler closed");
    });

    let peer_a_cfg = NetworkingConfig {
        port: 8081,
        bootstrap_nodes: vec![format!("/ip4/127.0.0.1/udp/8080/quic-v1/p2p/{}", bootstrap_id.clone())],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: vec![],
        private_key_path: "./volume/test-concurrent/peer-a/pkey".to_string(),
        name: "test-concurrent/peer-a".to_string(),
    };
    let mut peer_a = networking::context::context_create(&peer_a_cfg).unwrap();
    let mut peer_clone = peer_a.clone();

    let peer_b_cfg = NetworkingConfig {
        port: 8082,
        bootstrap_nodes: vec![format!("/ip4/127.0.0.1/udp/8080/quic-v1/p2p/{}", bootstrap_id_clone_clone.clone())],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: vec![],
        private_key_path: "./volume/test-concurrent/peer-b/pkey".to_string(),
        name: "test-concurrent/peer-b".to_string(),
    };
    let mut peer_b = networking::context::context_create(&peer_b_cfg).unwrap();

    let peer_c_cfg = NetworkingConfig {
        port: 8083,
        bootstrap_nodes: vec![format!("/ip4/127.0.0.1/udp/8080/quic-v1/p2p/{}", bootstrap_id_clone_clone.clone())],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: vec![],
        private_key_path: "./volume/test-concurrent/peer-c/pkey".to_string(),
        name: "test-concurrent/peer-c".to_string(),
    };
    let mut peer_c = networking::context::context_create(&peer_c_cfg).unwrap();
    tokio::spawn(async move {
        let mut s = peer_b.send(format!("send from {}", peer_b.id).as_bytes().to_vec(), bootstrap_id_clone_clone.clone(), protocol_clone_clone.clone(), 0).await.expect(&format!("{}: can't send message", peer_b.id));
        s.flush().await.expect("can't flush stream");
        s.close().await.expect("can't close stream");
        let buf = &mut Vec::new();
        let _ = s.read_to_end(buf).await.expect("can't read from stream");
        
        let msg = String::from_utf8_lossy(buf);
        println!("{}: Received message: {}", peer_b.id, msg);
    });
    tokio::spawn(async move {
        let mut s = peer_c.send(format!("send from {}", peer_c.id).as_bytes().to_vec(), bootstrap_id_clone_clone_clone.clone(), protocol_clone_clone_clone.clone(), 0).await.expect(&format!("{}: can't send message", peer_c.id));
        s.flush().await.expect("can't flush stream");
        s.close().await.expect("can't close stream");
        let buf = &mut Vec::new();
        let _ = s.read_to_end(buf).await.expect("can't read from stream");
        
        let msg = String::from_utf8_lossy(buf);
        println!("{}: Received message: {}", peer_c.id, msg);
    });

    tokio::spawn(async move {
        let mut s = peer_a.send(format!("1 - send from {}", peer_a.id).as_bytes().to_vec(), bootstrap_id.clone(), protocol.clone(), 0).await.expect(&format!("{}: can't send message", peer_a.id));
        s.flush().await.expect("can't flush stream");
        s.close().await.expect("can't close stream");
        
        let buf = &mut Vec::new();
        let _ = s.read_to_end(buf).await.expect("can't read from stream");
        
        let msg = String::from_utf8_lossy(buf);
        println!("{}: Received message: {}", peer_a.id, msg);
    });
    
    // tokio::spawn(async move {
    //     let mut s = peer_clone.send(format!("2 - send from {}", peer_clone.id).as_bytes().to_vec(), bootstrap_id_clone.clone(), protocol_clone.clone(), 0).await.expect(&format!("{}: can't send message", peer_clone.id));
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

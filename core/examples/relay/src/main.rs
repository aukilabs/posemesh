use networking::libp2p::{Networking, NetworkingConfig};
use tokio::{self, select};
use futures::{AsyncReadExt, StreamExt};

/*
    * This is a simple example of a relay node. It help relaying messages between private peers.
    * Usage: cargo run --package relay
    * Example: cargo run --package relay
 */
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let relay_cfg = &NetworkingConfig{
        port: 18803,
        bootstrap_nodes: vec![],
        enable_relay_server: true,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: None,
        private_key_path: Some("./volume/relay-example/relay/pkey".to_string()),
        name: "relay-example/relay".to_string(),
        enable_websocket: true,
        enable_webrtc: true,
        namespace: None,
    };
    let mut relay = Networking::new(relay_cfg)?;
    let protocol = "/chat".to_string();
    let mut chat_handler = relay.client.set_stream_handler(protocol).await.unwrap();

    loop {
        let relay_events = relay.event_receiver.clone();
        let mut relay_events = relay_events.lock().await;
        select! {
            Some((_, stream)) = chat_handler.next() => {
                let mut stream = stream;
                let mut buf = [0u8; 100];

                loop {
                    let read = stream.read(&mut buf).await?;
                    if read == 0 {
                        break;
                    }
                    println!("Received message: {:?}", String::from_utf8_lossy(&buf[..read]));
                }
            }
            e = relay_events.next() => {
                println!("Received relay event: {:?}", e);
            }
            else => break
        }
    }

    Ok(())
}

use posemesh_networking::{context, network, event};
use tokio::{self, signal, select};
use futures::AsyncReadExt;

/*
    * This is a simple example of a relay node. It will connect to a set of bootstraps and relay messages between them.
    * Usage: cargo run --example relay --features rust <port> <name> [private_key_path]
    * Example: cargo run --example relay --features rust 18804 relay
 */
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <port> <name> [private_key_path]", args[0]);
        return Ok(());
    }
    let port = args[1].parse::<u16>().unwrap();
    let name = args[2].clone();
    let mut private_key_path = format!("./volume/{}/pkey", name);
    if args.len() == 4 {
        private_key_path = args[3].clone();
    }

    let cfg = &network::NetworkingConfig{
        port: port,
        bootstrap_nodes: vec![],
        enable_relay_server: true,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: "".to_string(),
        private_key_path: private_key_path,
        name: name,
        node_capabilities: vec![],
        node_types: vec!["relay".to_string()],
    };
    let mut c = context::context_create(cfg)?;
    c.set_stream_handler("/chat".to_string()).await.unwrap();
    
    loop {
        select! {
            _ = signal::ctrl_c() => {
                break;
            }
            e = c.poll() => {
                match e {
                    Some(e) => {
                        match e {
                            event::Event::MessageReceived { peer, stream, .. } => {
                                let mut buf = Vec::new();
                                let mut s = stream;
                                s.read_to_end(&mut buf).await?;
                                println!("Received message from {}: {:?}", peer, String::from_utf8_lossy(&buf));
                            }
                            _ => {}
                        }
                    }
                    None => break
                }
            }
            else => break
        }
    }

    Ok(())
}

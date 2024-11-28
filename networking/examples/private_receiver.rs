use futures::future::select;
use posemesh_networking::{context, network};
use tokio::{runtime::Runtime, signal};

/*
    * This is a simple client that registers with a relay server.
    * Other clients can send messages to this client by sending messages to the relay server.

    * Usage: cargo run --example receiver --features rust <port> <name> <bootstraps> [private_key_path]
    * Example: cargo run --example receiver --features rust 0 relay_client /ip4/54.67.15.233/udp/18804/quic-v1/p2p/12D3KooWBMyph6PCuP6GUJkwFdR7bLUPZ3exLvgEPpR93J52GaJg
*/
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!("Usage: {} <port> <name> <bootstraps> [private_key_path]", args[0]);
        return Ok(());
    }
    let port = args[1].parse::<u16>().unwrap();
    let name = args[2].clone();
    let bootstraps = args[3].split(",").map(|s| s.to_string()).collect::<Vec<String>>();
    let mut private_key_path = format!("./volume/{}/pkey", name);
    if args.len() == 5 {
        private_key_path = args[4].clone();
    }

    let cfg = &network::NetworkingConfig{
        port: port,
        bootstrap_nodes: bootstraps.clone(),
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: bootstraps.clone(),
        private_key: "".to_string(),
        private_key_path: private_key_path,
        name: name,
        node_capabilities: vec![],
        node_types: vec!["client".to_string()],
    };
    
    let runtime = Runtime::new()?;
    runtime.block_on(async {
        let mut c = context::context_create(cfg).unwrap();

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
    });

    Ok(())
}

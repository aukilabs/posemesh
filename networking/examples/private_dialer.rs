use posemesh_networking::{context, network};
use tokio::{self, select, io::{self, AsyncBufReadExt, BufReader}};

/*
    * This is a simple chat client that sends messages to a peer.
    * Usage: cargo run --example sender --features rust <port> <name> <bootstraps> [private_key_path]
    * Example: cargo run --example sender --features rust 0 rust_client /ip4/54.67.15.233/udp/18804/quic-v1/p2p/12D3KooWBMyph6PCuP6GUJkwFdR7bLUPZ3exLvgEPpR93J52GaJg
*/
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!("Usage: {} <port> <name> <bootstraps>", args[0]);
        return Ok(());
    }
    let port = args[1].parse::<u16>().unwrap();
    let name = args[2].clone();
    let bootstraps = args[3].split(",").map(|s| s.to_string()).collect::<Vec<String>>();
    let private_key_path = format!("./volume/{}/pkey", name);

    let cfg = &network::NetworkingConfig{
        port: port,
        bootstrap_nodes: bootstraps.clone(),
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: bootstraps.clone(),
        private_key: vec![],
        private_key_path: private_key_path,
        name: name,
        node_capabilities: vec![],
        node_types: vec!["client".to_string()],
    };
    let mut c = context::context_create(cfg)?;

    let mut stdin = BufReader::new(io::stdin()).lines();

    loop {
        select! {
            line = stdin.next_line() => {
                let line = line.unwrap().unwrap();
                let s = line.split(":").collect::<Vec<&str>>();
                if s.len() == 2 {
                    let dest_peer = s[0].to_string();
                    let msg = s[1].to_string();
                    match c.send(msg.as_bytes().to_vec(), dest_peer.clone(), "/chat".to_string()).await {
                        Ok(_) => {
                            println!("Sent message to {}: {:?}", dest_peer, msg);
                        }
                        Err(e) => {
                            eprintln!("Error sending message: {:?}", e);
                        }
                    }

                    c.send(msg.as_bytes().to_vec(), dest_peer.clone(), "/undefined".to_string()).await.expect_err("Expected error");
                }
            }
        }
    }
}

use posemesh_networking::{context, network};
use tokio::{self, select};
use tokio::io::{self, AsyncBufReadExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!("Usage: {} <port> <name> <test_peer_id> [private_key_path]", args[0]);
        return Ok(());
    }
    let port = args[1].parse::<u16>().unwrap();
    let name = args[2].clone();
    let test_peer = args[3].clone();
    let mut private_key_path = "./volume/pkey".to_string();
    if args.len() == 5 {
        private_key_path = args[4].clone();
    }

    let cfg = &network::NetworkingConfig{
        port: port,
        bootstrap_nodes: vec![],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: true,
        relay_nodes: vec![],
        private_key: "".to_string(),
        private_key_path: private_key_path,
        name: name,
        node_capabilities: vec![],
        node_types: vec!["relay".to_string()],
    };
    let mut c = context::context_create(cfg)?;
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    loop {
        select! {
            line = stdin.next_line() => {
                let line = line.unwrap().unwrap();
                let _ = c.send(line.as_bytes().to_vec(), test_peer.clone(), "/chat".to_string())
                    .await?;
            }
        }
    }
    Ok(())
}

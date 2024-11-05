use posemesh_networking::{context, network};
use tokio::{self, select};
use tokio::io::{self, AsyncBufReadExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!("Usage: {} <port> <name> <bootstraps> [private_key_path]", args[0]);
        return Ok(());
    }
    let port = args[1].parse::<u16>().unwrap();
    let name = args[2].clone();
    let bootstraps = args[3].split(",").map(|s| s.to_string()).collect::<Vec<String>>();
    let mut private_key_path = "./volume/pkey".to_string();
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
    let mut c = context::context_create(cfg)?;

    let mut stdin = io::BufReader::new(io::stdin()).lines();

    loop {
        select! {
            line = stdin.next_line() => {
                let line = line.unwrap().unwrap();
                let s = line.split(":").collect::<Vec<&str>>();
                if s.len() == 2 {
                    let dest_peer = s[0].to_string();
                    let msg = s[1].to_string();
                    let _ = c.send(msg.as_bytes().to_vec(), dest_peer.clone(), "/chat".to_string())
                    .await?;
                }
            }
        }
    }
}

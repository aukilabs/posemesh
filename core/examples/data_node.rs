use jsonwebtoken::{decode, DecodingKey,Validation, Algorithm};
use libp2p::Stream;
use networking::{context, event, network::{self, Node}};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use tokio::{self, select, signal};
use futures::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use std::{collections::HashMap, fs::OpenOptions, io::Write};
use protobuf::{task::{self, StoreDataOutputV1}, domain_data};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct TaskTokenClaim {
    task_name: String,
    job_id: String,
    sender: String,
    receiver: String,
    // exp: usize,
}

fn decode_jwt(token: &str) -> Result<TaskTokenClaim, Box<dyn std::error::Error + Send + Sync>> {
    let token_data = decode::<TaskTokenClaim>(token, &DecodingKey::from_secret("secret".as_ref()), &Validation::new(Algorithm::HS256))?;
    Ok(token_data.claims)
}

async fn handshake(stream: &mut Stream) -> Result<TaskTokenClaim, Box<dyn std::error::Error + Send + Sync>> {
    let mut length_buf = [0u8; 4];
    stream.read_exact(&mut length_buf).await?;

    let length = u32::from_be_bytes(length_buf) as usize;
    let mut buffer = vec![0u8; length];
    stream.read_exact(&mut buffer).await?;
        
    let header = deserialize_from_slice::<task::DomainClusterHandshake>(&buffer)?;
    println!("Received handshake: {:?}", header);

    decode_jwt(header.access_token.as_str())
}

async fn store_data_v1(base_path: String, mut stream: Stream, mut c: context::Context) {
    tokio::spawn(async move {
        let claim = handshake(&mut stream).await.expect("Failed to handshake");
        let job_id = claim.job_id.clone();
        c.subscribe(job_id.clone()).await.expect("Failed to subscribe to job");
        let mut data_ids = Vec::<String>::new();

        loop {
            let mut length_buf = [0u8; 4];
            let has = stream.read(&mut length_buf).await.expect("Failed to read length");
            if has == 0 {
                break;
            }
    
            let length = u32::from_be_bytes(length_buf) as usize;
    
            // Read the data in chunks
            let mut buffer = vec![0u8; length];
            stream.read_exact(&mut buffer).await.expect("Failed to read buffer");
    
            let mut metadata = deserialize_from_slice::<domain_data::DomainDataMetadata>(&buffer).expect("Failed to deserialize metadata");
    
            let data_id = Uuid::new_v4().to_string();
    
            let mut f = OpenOptions::new().append(true).create(true).open(format!("{}/output/domain_data/{}", base_path, metadata.name)).expect("Failed to open file");
            let default_chunk_size = 3 * 1024;
            let mut read_size: usize = 0;
            let data_size = metadata.size as usize;
            println!("Storing data with id: {}, size: {}", data_id, metadata.size);
            loop {
                // TODO: add timeout so stream wont be idle for too long
                let chunk_size = if data_size - read_size > default_chunk_size { default_chunk_size } else { data_size - read_size };
                if chunk_size == 0 {
                    metadata.hash = data_id.clone();
                    let m_buf = serialize_into_vec(&metadata).expect("Failed to serialize metadata");
                    let mut length_buf = [0u8; 4];
                    let length = m_buf.len() as u32;
                    length_buf.copy_from_slice(&length.to_be_bytes());
                    
                    stream.write_all(&length_buf).await.expect("Failed to write length");
                    
                    stream.write_all(&m_buf).await.expect("Failed to write metadata");
                    stream.flush().await.expect("Failed to flush");
    
                    data_ids.push(data_id.clone());
    
                    f.flush().expect("Failed to flush file");
                    println!("Stored data with id: {}, size: {}", data_id, metadata.size);
                    break;
                }
                let mut buffer = vec![0u8; chunk_size];
                stream.read_exact(&mut buffer).await.expect("Failed to read buffer");
    
                read_size+=chunk_size;
    
                f.write_all(&buffer).expect("Failed to write buffer");
                println!("read {}/{}", read_size, data_size);
            }
        }
    
        let event = task::Task {
            name: claim.task_name.clone(),
            receiver: claim.receiver.clone(),
            sender: claim.sender.clone(),
            endpoint: "/store/v1".to_string(),
            status: task::Status::DONE,
            access_token: "".to_string(),
            job_id: job_id.clone(),
            output: Some(task::Any {
                type_url: "StoreDataOutputV1".to_string(),
                value: serialize_into_vec(&StoreDataOutputV1 {
                    ids: data_ids.clone(),
                }).expect("Failed to serialize store data output"),
            }),
        };
        c.publish(job_id.clone(), serialize_into_vec(&event).expect("Failed to serialize task update")).await.expect("failed to publish task update");
        println!("Published task update");
    });
}

/*
    * This is a simple example of a data node. It will connect to the domain manager and store and retrieve domain data.
    * Usage: cargo run --example data --features rust <port> <name> <domain_manager> 
    * Example: cargo run --example data --features rust 18804 data 
 */
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <port> <name> <domain_manager>", args[0]);
        return Ok(());
    }
    let port = args[1].parse::<u16>().unwrap();
    let name = args[2].clone();
    let base_path = format!("./volume/{}", name);
    let domain_manager = args[3].clone();
    let private_key_path = format!("{}/pkey", base_path);

    let cfg = &network::NetworkingConfig{
        port: port,
        bootstrap_nodes: vec![domain_manager.clone()],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: vec![],
        private_key_path: private_key_path,
        name: name,
    };
    let mut c = context::context_create(cfg)?;
    c.set_stream_handler("/store/v1".to_string()).await.unwrap();
    let mut nodes: HashMap<String, Node> = HashMap::new();

    let domain_manager_id = domain_manager.split("/").last().unwrap().to_string();

    loop {
        select! {
            _ = signal::ctrl_c() => {
                break;
            }
            e = c.poll() => {
                match e {
                    Some(e) => {
                        match e {
                            event::Event::StreamMessageReceivedEvent { peer, msg_reader, protocol } => {
                                if protocol.to_string() == "/store/v1" {
                                    store_data_v1(base_path.to_string(), msg_reader, c.clone()).await;
                                }
                            }
                            event::Event::NewNodeRegistered { node } => {
                                println!("New node registered: {:?}", node);
                                nodes.insert(node.id.clone(), node);
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

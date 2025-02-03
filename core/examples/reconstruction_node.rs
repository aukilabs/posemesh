use jsonwebtoken::{decode, DecodingKey,Validation, Algorithm};
use libp2p::Stream;
use networking::{context, event, network::{self, Node}};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use tokio::{self, select, signal, time::{sleep, Duration}};
use futures::{io::BufReader, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, StreamExt};
use uuid::Uuid;
use std::{borrow::BorrowMut, collections::HashMap, fs::{read, OpenOptions}, io::Write};
use protobuf::{task::{self, StoreDataOutputV1, DomainClusterHandshake, LocalRefinementOutputV1, Task}, domain_data};
use sha2::{digest::crypto_common::rand_core::le, Digest, Sha256};
use hex;
use std::{io, fs};
use quick_protobuf_codec::Codec;
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

async fn local_refinement_v1(mut stream: Stream, mut c: context::Context) {
    let claim = handshake(&mut stream).await.expect("Failed to handshake");
    let job_id = claim.job_id.clone();
    c.subscribe(job_id.clone()).await.expect("Failed to subscribe to job");
    tokio::spawn(async move {
        let mut length_buf = [0u8; 4];
        stream.read_exact(&mut length_buf).await.expect("Failed to read length");

        let length = u32::from_be_bytes(length_buf) as usize;
        let mut buffer = vec![0u8; length];
        stream.read_exact(&mut buffer).await.expect("Failed to read buffer");
            
        let input = deserialize_from_slice::<task::LocalRefinementInputV1>(&buffer).expect("Failed to deserialize local refinement input");

        println!("Start executing {}", claim.task_name);

        let sleep_time = rand::random::<u64>() % 20;
        sleep(Duration::from_secs(sleep_time)).await;

        let output = LocalRefinementOutputV1 {
            recording_id: input.recording_id.clone(),
            result_id: Uuid::new_v4().to_string(),
        };
        let event = task::Task {
            name: claim.task_name.clone(),
            receiver: claim.receiver.clone(),
            sender: claim.sender.clone(),
            endpoint: "/local-refinement/v1".to_string(),
            status: task::Status::DONE,
            access_token: "".to_string(),
            job_id: job_id.clone(),
            output: Some(task::Any {
                type_url: "LocalRefinementOutputV1".to_string(),
                value: serialize_into_vec(&output).expect("Failed to serialize local refinement output"),
            }),
        };
        let buf = serialize_into_vec(&event).expect("failed to serialize task update");
        c.publish(job_id.clone(), buf).await.expect("failed to publish task update");
    });
}

async fn global_refinement_v1(mut stream: Stream, mut c: context::Context) {
    let claim = handshake(&mut stream).await.expect("Failed to handshake");
    let job_id = claim.job_id.clone();
    c.subscribe(job_id.clone()).await.expect("Failed to subscribe to job");
    tokio::spawn(async move {
        let mut length_buf = [0u8; 4];
        stream.read_exact(&mut length_buf).await.expect("Failed to read length");

        let length = u32::from_be_bytes(length_buf) as usize;
        let mut buffer = vec![0u8; length];
        stream.read_exact(&mut buffer).await.expect("Failed to read buffer");
            
        let input = deserialize_from_slice::<task::GlobalRefinementInputV1>(&buffer).expect("Failed to deserialize global refinement input");

        sleep(Duration::from_secs(10)).await;

        println!("Received global refinement input: {:?}", input);

        let event = task::Task {
            name: claim.task_name.clone(),
            receiver: claim.receiver.clone(),
            sender: claim.sender.clone(),
            endpoint: "/global-refinement/v1".to_string(),
            status: task::Status::DONE,
            access_token: "".to_string(),
            job_id: job_id.clone(),
            output: None,
        };
        let buf = serialize_into_vec(&event).expect("failed to serialize task update");
        c.publish(job_id.clone(), buf).await.expect("failed to publish task update");
    });
}
/*
    * This is a simple example of a reconstruction node. It will connect to a set of bootstraps and execute reconstruction jobs.
    * Usage: cargo run --example reconstruction --features rust <port> <name> <domain_manager> 
    * Example: cargo run --example reconstruction --features rust 18808 reconstruction 
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
        // node_capabilities: vec![],
        // node_types: vec!["relay".to_string()],
    };
    let mut c = context::context_create(cfg)?;
    c.set_stream_handler("/local-refinement/v1".to_string()).await.unwrap();
    c.set_stream_handler("/global-refinement/v1".to_string()).await.unwrap();
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
                                if protocol.to_string() == "/local-refinement/v1" {
                                    local_refinement_v1(msg_reader, c.clone()).await;
                                } else if protocol.to_string() == "/global-refinement/v1" {
                                    global_refinement_v1(msg_reader, c.clone()).await;
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

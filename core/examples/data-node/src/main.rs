use domain::{cluster::DomainCluster, datastore::{common::Datastore, fs::FsDatastore, metadata::MetadataStore, remote::{CONSUME_DATA_PROTOCOL_V1, PRODUCE_DATA_PROTOCOL_V1}}, message::read_prefix_size_message, protobuf::{domain_data::{Metadata, Query}, task::{ConsumeDataInputV1, DomainClusterHandshake, Status, Task}}};
use jsonwebtoken::{decode, DecodingKey,Validation, Algorithm};
use libp2p::Stream;
use networking::{event, libp2p::{Networking, NetworkingConfig, Node}};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use tokio::{self, select, signal::unix::{signal, SignalKind}};
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use std::{fs::{self, OpenOptions}, io::{Read, Write}};
use serde::{Deserialize, Serialize};

async fn shutdown_signal() {
    let mut term_signal = signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
    let mut int_signal = signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

    select! {
        _ = term_signal.recv() => println!("Received SIGTERM, exiting..."),
        _ = int_signal.recv() => println!("Received SIGINT, exiting..."),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskTokenClaim {
    domain_id: String,
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
    let header = read_prefix_size_message::<DomainClusterHandshake>(stream).await?;
    decode_jwt(header.access_token.as_str())
}

fn store_data_v1(mut stream: Stream, mut c: Networking, fs_datastore: FsDatastore) {
    tokio::spawn(async move {
        let claim = handshake(&mut stream).await.expect("Failed to handshake");
        let job_id = claim.job_id.clone();
        c.client.subscribe(job_id.clone()).await.expect("Failed to subscribe to job");

        let task = Task {
            name: claim.task_name.clone(),
            receiver: Some(claim.receiver.clone()),
            sender: claim.sender.clone(),
            endpoint: PRODUCE_DATA_PROTOCOL_V1.to_string(),
            status: Status::STARTED,
            access_token: None,
            job_id: claim.job_id.clone(),
            output: None,
        };
        let buf = serialize_into_vec(&task).expect("Failed to serialize task update");
        c.client.publish(job_id.clone(), buf).await.expect("Failed to publish task update");
        let mut data_ids = Vec::<String>::new();
        let domain_id = claim.domain_id.clone();

        let mut producer = fs_datastore.produce(domain_id).await.expect("Failed to create producer");

        loop {
            let mut length_buf = [0u8; 4];
            if res.is_err() {
                let err = res.err().unwrap();
                if err.kind() == std::io::ErrorKind::UnexpectedEof {
                    return Ok(());
                } else {
                    return Err(err.into());
                }
            }
            let length = u32::from_be_bytes(length_buf) as usize;
    
            // Read the data in chunks
            let mut buffer = vec![0u8; length];
            stream.read_exact(&mut buffer).await.expect("Failed to read buffer");
            let metadata = deserialize_from_slice::<Metadata>(&buffer).expect("Failed to deserialize metadata");
            println!("Received buffer: {:?}", metadata);
    
            let data_id = metadata.id.expect("Failed to get data id");

            let default_chunk_size = 5 * 1024;
            let mut read_size: usize = 0;
            let data_size = metadata.size as usize;
            let mut content = Vec::<u8>::with_capacity(data_size);
            loop {
                // TODO: add timeout so stream wont be idle for too long
                let chunk_size = if data_size - read_size > default_chunk_size { default_chunk_size } else { data_size - read_size };
                if chunk_size == 0 {
                    data_ids.push(data_id.clone());
                    producer.push(&Data {
                        domain_id: domain_id.clone(),
                        metadata: metadata.clone(),
                        content,
                    }).await.expect("Failed to store data");
                    println!("Stored data: {}, size: {}", metadata.name, metadata.size);
                    stream.write_all(&length_buf).await.expect("Failed to write length");
                    
                    stream.write_all(&buffer).await.expect("Failed to write metadata");
                    stream.flush().await.expect("Failed to flush");
                    break;
                }
                let mut buffer = vec![0u8; chunk_size];
                stream.read_exact(&mut buffer).await.expect("Failed to read buffer");
    
                read_size+=chunk_size;
    
                content.extend_from_slice(&buffer);
                println!("Received chunk: {}/{}", read_size, metadata.size);
            }
        }
    });
}

async fn serve_data_v1(mut stream: Stream, mut c: Networking, fs_datastore: FsDatastore) {
    let header = handshake(&mut stream).await.expect("Failed to handshake");
    c.client.subscribe(header.job_id.clone()).await.expect("Failed to subscribe to job");
    let input = deserialize_from_slice::<ConsumeDataInputV1>(&buf).expect("Failed to deserialize consume data input");
    let mut consumer = fs_datastore.consume(header.domain_id.clone(), input.query, input.keep_alive).await;
    loop {
        select! {
            result = consumer.next() => {
                match result {
                    Some(Ok(data)) => {
                        stream.write_all(&prefix_size_message(data.metadata)).await.expect("Failed to write data");
                        stream.write_all(&data.content).await.expect("Failed to write data");
                        stream.flush().await.expect("Failed to flush");
                        println!("Served data: {}, size: {}", data.metadata.name, data.metadata.size);
                    }
                    Some(Err(e)) => {
                        println!("Error: {:?}", e);
                        task.status = Status::FAILED;
                        let buf = serialize_into_vec(&task).expect("Failed to serialize task update");
                        c.client.publish(header.job_id.clone(), buf).await.expect("Failed to publish task update");
                        return;
                    }
                    None => break
                }
            }
        }
    }

    if !input.keep_alive {
        let task = Task {
            name: header.task_name.clone(),
            receiver: Some(header.receiver.clone()),
            sender: header.sender.clone(),
            endpoint: CONSUME_DATA_PROTOCOL_V1.to_string(),
            status: Status::DONE,
            access_token: None,
            job_id: header.job_id.clone(),
            output: None,
        };
        let buf = serialize_into_vec(&task)?;
        c.client.publish(header.job_id.clone(), buf).await?;
    }

    Ok(())
}
/*
    * This is a simple example of a data node. It will connect to the domain manager and store and retrieve domain data.
    * Usage: cargo run --package data-node <port> <name> <domain_manager> 
    * Example: cargo run --package data-node data 18804 data /ip4/127.0.0.1/udp/18800/quic-v1/p2p/12D3KooWDHaDQeuYeLM8b5zhNjqS7Pkh7KefqzCpDGpdwj5iE8pq
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

    let domain_manager_id = domain_manager.split("/").last().unwrap().to_string();
    let domain_cluster = DomainCluster::new(domain_manager.clone(), name, false, port, true, true, None, Some(private_key_path));
    let mut n = domain_cluster.peer;
    let mut produce_handler = n.client.set_stream_handler(PRODUCE_DATA_PROTOCOL_V1.to_string()).await.unwrap();
    let mut consume_handler = n.client.set_stream_handler(CONSUME_DATA_PROTOCOL_V1.to_string()).await.unwrap();
    let conn_str = "postgres://postgres:postgres@localhost:5432/postgres";
    let path = format!("{}/output/domain_data", base_path);
    let _ = std::fs::remove_dir_all(path.clone());
    std::fs::create_dir_all(path.clone()).expect("Failed to create domain_data directory");

    let metadata_store = MetadataStore::new(conn_str, path.clone().as_str());
    let fs_datastore = FsDatastore::new(metadata_store);

    loop {
        select! {
            Some((_, stream)) = produce_handler.next() => {
                // let tx = tx.clone();
                let base_path = base_path.clone();
                let n = n.clone();
                tokio::spawn(async move {
                    if let Err(e) = store_data_v1(base_path, stream, n).await {
                        println!("Error storing data: {}", e);
                    }
                });
            }
            Some((_, stream)) = consume_handler.next() => {
                let base_path = base_path.clone();
                let n = n.clone();
                tokio::spawn(async move {
                    if let Err(e) = serve_data_v1(base_path, stream, n).await {
                        println!("Error serving data: {}", e);
                    }
                });
            }
            _ = shutdown_signal() => {
                println!("Received shutdown signal, exiting...");
                break;
            }
        }
    }

    println!("Exit");

    Ok(())
}

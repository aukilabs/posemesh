use domain::{datastore::remote::{CONSUME_DATA_PROTOCOL_V1, PRODUCE_DATA_PROTOCOL_V1}, protobuf::domain_data::Metadata};
use jsonwebtoken::{decode, DecodingKey,Validation, Algorithm};
use libp2p::Stream;
use networking::{context, event, network::{self, Node}};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use tokio::{self, select, signal};
use futures::{channel::{self, mpsc::{channel, Receiver, Sender}}, lock::Mutex, AsyncReadExt, AsyncWriteExt, SinkExt, StreamExt};
use uuid::Uuid;
use std::{collections::HashMap, fs::{self, OpenOptions}, io::{Read, Write}, sync::Arc};
use protobuf::{task::{self, StoreDataOutputV1}, domain_data};
use serde::{de, Deserialize, Serialize};

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

    decode_jwt(header.access_token.as_str())
}

async fn store_data_v1(base_path: String, mut stream: Stream, mut c: context::Context, mut notify: Sender<String>) {
    tokio::spawn(async move {
        let claim = handshake(&mut stream).await.expect("Failed to handshake");
        let job_id = claim.job_id.clone();
        c.subscribe(job_id.clone()).await.expect("Failed to subscribe to job");
        let mut data_ids = Vec::<String>::new();

        println!("Authenticated! {:?}", claim);

        loop {
            let mut length_buf = [0u8; 4];
            
             match stream.read_exact(&mut length_buf).await {
                Ok(_) => 4,
                Err(e) => {
                    eprintln!("Failed to read message length: {:?}", e);
                    break;
                },
            };
    
            let length = u32::from_be_bytes(length_buf) as usize;
    
            // Read the data in chunks
            let mut buffer = vec![0u8; length];
            stream.read_exact(&mut buffer).await.expect("Failed to read buffer");
            let metadata = deserialize_from_slice::<Metadata>(&buffer).expect("Failed to deserialize metadata");
    
            let data_id = metadata.id.expect("Failed to get data id");
    
            // create domain_data directory if it doesn't exist
            let path = format!("{}/output/domain_data/{}", base_path, data_id.clone());
            std::fs::create_dir_all(path.clone()).expect("Failed to create domain_data directory");
            let mut metadata_file = OpenOptions::new().append(true).create(true).open(format!("{}/metadata.bin", path.clone())).expect("Failed to open file");
            metadata_file.write_all(&buffer).expect("Failed to write metadata");
            metadata_file.flush().expect("Failed to flush file");

            let mut content_file = OpenOptions::new().append(true).create(true).open(format!("{}/content.bin", path.clone())).expect("Failed to open file");
            let default_chunk_size = 3 * 1024;
            let mut read_size: usize = 0;
            let data_size = metadata.size as usize;
            loop {
                // TODO: add timeout so stream wont be idle for too long
                let chunk_size = if data_size - read_size > default_chunk_size { default_chunk_size } else { data_size - read_size };
                if chunk_size == 0 {
                    stream.write_all(&length_buf).await.expect("Failed to write length");
                    
                    stream.write_all(&buffer).await.expect("Failed to write metadata");
                    stream.flush().await.expect("Failed to flush");
    
                    data_ids.push(data_id.clone());
    
                    content_file.flush().expect("Failed to flush file");
                    println!("Stored data: {}, size: {}", metadata.name, metadata.size);
                    notify.send(path.clone()).await.expect("Failed to send notification");
                    break;
                }
                let mut buffer = vec![0u8; chunk_size];
                stream.read_exact(&mut buffer).await.expect("Failed to read buffer");
    
                read_size+=chunk_size;
    
                content_file.write_all(&buffer).expect("Failed to write buffer");
                println!("Wrote chunk: {}/{}", read_size, metadata.size);
            }
        }
    
        // let event = task::Task {
        //     name: claim.task_name.clone(),
        //     receiver: claim.receiver.clone(),
        //     sender: claim.sender.clone(),
        //     endpoint: PRODUCE_DATA_PROTOCOL_V1.to_string(),
        //     status: task::Status::DONE,
        //     access_token: "".to_string(),
        //     job_id: job_id.clone(),
        //     output: Some(task::Any {
        //         type_url: "StoreDataOutputV1".to_string(),
        //         value: serialize_into_vec(&StoreDataOutputV1 {
        //             ids: data_ids.clone(),
        //         }).expect("Failed to serialize store data output"),
        //     }),
        // };
        // c.publish(job_id.clone(), serialize_into_vec(&event).expect("Failed to serialize task update")).await.expect("failed to publish task update");
        // println!("Published task update");
    });
}

async fn serve_data_v1(base_path: String, mut stream: Stream, mut c: context::Context, listener: Arc<Mutex<Receiver<String>>>) {
    // create domain_data directory if it doesn't exist
    std::fs::create_dir_all(format!("{}/output/domain_data", base_path)).expect("Failed to create domain_data directory");
    tokio::spawn(async move {
        let listener = listener.clone();
        // keep loading files from domain_data directory, /domain_data/<data_id>/metadata.bin + /domain_data/<data_id>/content.bin
        // write metadata.bin into protobuf and send it to the stream, send content.bin in chunks to the stream

        // load directories from domain_data directory
        let paths = std::fs::read_dir(format!("{}/output/domain_data", base_path)).expect("Failed to read domain_data directory");
        for path in paths {
            let path = path.expect("Failed to read path").path();
            let data_id = path.file_name().expect("Failed to get file name").to_str().expect("Failed to convert to str").to_string();
            let metadata_path = format!("{}/metadata.bin", path.to_str().expect("Failed to convert to str"));
            let content_path = format!("{}/content.bin", path.to_str().expect("Failed to convert to str"));

            let metadata_buf = std::fs::read(metadata_path).expect("Failed to read metadata");
            let mut length_buf = [0u8; 4];
            let length = metadata_buf.len() as u32;
            length_buf.copy_from_slice(&length.to_be_bytes());
            stream.write_all(&length_buf).await.expect("Failed to write length");
            stream.write_all(&metadata_buf).await.expect("Failed to write metadata");

            let metadata = deserialize_from_slice::<domain_data::DomainDataMetadata>(&metadata_buf).expect("Failed to deserialize metadata");

            let mut f = fs::File::open(content_path).expect("Failed to open file");
            let mut written = 0;
            let chunk_size = 7 * 1024;
            loop {
                let mut buf = vec![0; chunk_size];
                let n = f.read(&mut buf).expect("Failed to read chunk");
                if n == 0 {
                    break;
                }
                written += n;
                println!("Wrote chunk: {}/{}", written, metadata.size);
                stream.write_all(&buf[..n]).await.expect("cant write chunk");
                stream.flush().await.expect("cant flush chunk");
            }
        }

        let mut listener = listener.lock().await;
        // send file when listener has next notification
        loop {
            match listener.next().await {
                Some(path) => {
                    let metadata_path = format!("{}/metadata.bin", path.clone());
                    let metadata_buf = std::fs::read(metadata_path.clone()).expect("Failed to read metadata");
                    let mut length_buf = [0u8; 4];
                    let length = metadata_buf.len() as u32;
                    length_buf.copy_from_slice(&length.to_be_bytes());
                    stream.write_all(&length_buf).await.expect("Failed to write length");
                    stream.write_all(&metadata_buf).await.expect("Failed to write metadata");

                    let metadata = deserialize_from_slice::<domain_data::DomainDataMetadata>(&metadata_buf).expect("Failed to deserialize metadata");

                    let content_path = format!("{}/content.bin", path.clone());
                    let mut f = fs::File::open(content_path).expect("Failed to open file");
                    let mut written = 0;
                    let chunk_size = 7 * 1024;
                    loop {
                        let mut buf = vec![0; chunk_size];
                        let n = f.read(&mut buf).expect("Failed to read chunk");
                        if n == 0 {
                            break;
                        }
                        written += n;
                        println!("Wrote chunk: {}/{}", written, metadata.size);
                        stream.write_all(&buf[..n]).await.expect("cant write chunk");
                        stream.flush().await.expect("cant flush chunk");
                    }
                }
                None => break,
            }
        }
    });
}
/*
    * This is a simple example of a data node. It will connect to the domain manager and store and retrieve domain data.
    * Usage: cargo run --example data --features rust <port> <name> <domain_manager> 
    * Example: cargo run --example data --features rust 18804 data /ip4/127.0.0.1/udp/18800/quic-v1/p2p/12D3KooWDHaDQeuYeLM8b5zhNjqS7Pkh7KefqzCpDGpdwj5iE8pq
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
    c.set_stream_handler(PRODUCE_DATA_PROTOCOL_V1.to_string()).await.unwrap();
    c.set_stream_handler(CONSUME_DATA_PROTOCOL_V1.to_string()).await.unwrap();
    let mut nodes: HashMap<String, Node> = HashMap::new();

    let domain_manager_id = domain_manager.split("/").last().unwrap().to_string();
    let (tx, rx) = channel::<String>(100);
    let rx = Arc::new(Mutex::new(rx));
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
                                if protocol.to_string() == PRODUCE_DATA_PROTOCOL_V1 {
                                    store_data_v1(base_path.to_string(), msg_reader, c.clone(), tx.clone()).await;
                                } else if protocol.to_string() == CONSUME_DATA_PROTOCOL_V1 {
                                    serve_data_v1(base_path.to_string(), msg_reader, c.clone(), rx.clone()).await;
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

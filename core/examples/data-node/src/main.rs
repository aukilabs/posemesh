use domain::{cluster::DomainCluster, datastore::remote::{CONSUME_DATA_PROTOCOL_V1, PRODUCE_DATA_PROTOCOL_V1}, message::read_prefix_size_message, protobuf::{domain_data::{Metadata, Query}, task::{ConsumeDataInputV1, DomainClusterHandshake, Status, Task}}};
use jsonwebtoken::{decode, DecodingKey,Validation, Algorithm};
use libp2p::Stream;
use networking::{event, libp2p::{Networking, NetworkingConfig, Node}};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use tokio::{self, select};
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use std::{fs::{self, OpenOptions}, io::{Read, Write}};
use serde::{Deserialize, Serialize};

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
    let header = read_prefix_size_message::<DomainClusterHandshake>(stream).await?;
    decode_jwt(header.access_token.as_str())
}

fn store_data_v1(base_path: String, mut stream: Stream, mut c: Networking) {
    tokio::spawn(async move {
        let claim = handshake(&mut stream).await.expect("Failed to handshake");
        let job_id = claim.job_id.clone();
        c.client.subscribe(job_id.clone()).await.expect("Failed to subscribe to job");
        let mut data_ids = Vec::<String>::new();

        println!("Storing data for job: {:?}", claim.task_name);
        loop {
            let mut length_buf = [0u8; 4];
            stream.read_exact(&mut length_buf).await.expect("Failed to read length");
            let length = u32::from_be_bytes(length_buf) as usize;

            println!("Received length: {}", length);
    
            // Read the data in chunks
            let mut buffer = vec![0u8; length];
            stream.read_exact(&mut buffer).await.expect("Failed to read buffer");
            let metadata = deserialize_from_slice::<Metadata>(&buffer).expect("Failed to deserialize metadata");
            println!("Received buffer: {:?}", metadata);
    
            let data_id = metadata.id.expect("Failed to get data id");
    
            // create domain_data directory if it doesn't exist
            let path = format!("{}/output/domain_data/{}", base_path, data_id.clone());
            std::fs::create_dir_all(path.clone()).expect("Failed to create domain_data directory");
            let mut metadata_file = OpenOptions::new().append(true).create(true).open(format!("{}/metadata.bin", path.clone())).expect("Failed to open file");
            metadata_file.write_all(&buffer).expect("Failed to write metadata");
            metadata_file.flush().expect("Failed to flush file");

            let mut content_file = OpenOptions::new().append(true).create(true).open(format!("{}/content.bin", path.clone())).expect("Failed to open file");
            let default_chunk_size = 5 * 1024;
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
                    // notify.send(path.clone()).await.expect("Failed to send notification");
                    break;
                }
                let mut buffer = vec![0u8; chunk_size];
                stream.read_exact(&mut buffer).await.expect("Failed to read buffer");
    
                read_size+=chunk_size;
    
                content_file.write_all(&buffer).expect("Failed to write buffer");
                println!("Received chunk: {}/{}", read_size, metadata.size);
            }
        }
    });
}

async fn serve_data_v1(base_path: String, mut stream: Stream, mut c: Networking) {
    let header = handshake(&mut stream).await.expect("Failed to handshake");
    c.client.subscribe(header.job_id.clone()).await.expect("Failed to subscribe to job");
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf).await.expect("Failed to read stream");
    let input = deserialize_from_slice::<ConsumeDataInputV1>(&buf).expect("Failed to deserialize consume data input");

    std::fs::create_dir_all(format!("{}/output/domain_data", base_path)).expect("Failed to create domain_data directory");
    // let listener = listener.clone();
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

        let metadata = deserialize_from_slice::<Metadata>(&metadata_buf).expect("Failed to deserialize metadata");

        let mut f = fs::File::open(content_path).expect("Failed to open file");
        let mut written = 0;
        let chunk_size = 2 * 1024;
        loop {
            let mut buf = vec![0; chunk_size];
            let n = f.read(&mut buf).expect("Failed to read chunk");
            if n == 0 {
                break;
            }
            written += n;
            // println!("Served chunk: {}/{}", written, metadata.size);
            stream.write_all(&buf[..n]).await.expect("cant write chunk");
            stream.flush().await.expect("cant flush chunk");
        }
        println!("Served data: {}, size: {}", metadata.name, metadata.size);
    }

    if !input.keep_alive {
        let task = Task {
            name: header.task_name.clone(),
            receiver: header.receiver.clone(),
            sender: header.sender.clone(),
            endpoint: CONSUME_DATA_PROTOCOL_V1.to_string(),
            status: Status::DONE,
            access_token: "".to_string(),
            job_id: header.job_id.clone(),
            output: None,
        };
        let buf = serialize_into_vec(&task).expect("Failed to serialize task update");
        c.client.publish(header.job_id.clone(), buf).await.expect("Failed to publish task update");
    }
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
    let domain_cluster = DomainCluster::new(domain_manager.clone(), name, false, None, Some(private_key_path));
    let mut n = domain_cluster.peer;
    let mut produce_handler = n.client.set_stream_handler(PRODUCE_DATA_PROTOCOL_V1.to_string()).await.unwrap();
    let mut consume_handler = n.client.set_stream_handler(CONSUME_DATA_PROTOCOL_V1.to_string()).await.unwrap();
    let _ = std::fs::remove_dir_all(format!("{}/output/domain_data", base_path));

    loop {
        select! {
            Some((_, stream)) = produce_handler.next() => {
                // let tx = tx.clone();
                let base_path = base_path.clone();
                store_data_v1(base_path, stream, n.clone());
            }
            Some((_, stream)) = consume_handler.next() => {
                // let rx = rx.clone();
                let base_path = base_path.clone();
                tokio::spawn(serve_data_v1(base_path, stream, n.clone()));
            }
            else => break
        }
    }

    Ok(())
}

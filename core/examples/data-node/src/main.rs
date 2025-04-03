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

async fn store_data_v1(base_path: String, mut stream: Stream, mut c: Networking) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let claim = handshake(&mut stream).await?;
    let job_id = claim.job_id.clone();
    c.client.subscribe(job_id.clone()).await?;
    let mut data_ids = Vec::<String>::new();

    loop {
        let mut length_buf = [0u8; 4];
        let res = stream.read_exact(&mut length_buf).await;
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
        stream.read_exact(&mut buffer).await?;
        let metadata = deserialize_from_slice::<Metadata>(&buffer)?;
        println!("Received buffer: {:?}", metadata);

        let data_id = metadata.id.expect("Failed to get data id");

        // create domain_data directory if it doesn't exist
        let path = format!("{}/output/domain_data/{}", base_path, data_id.clone());
        std::fs::create_dir_all(path.clone())?;
        let mut metadata_file = OpenOptions::new().append(true).create(true).open(format!("{}/metadata.bin", path.clone()))?;
        metadata_file.write_all(&buffer)?;
        metadata_file.flush()?;

        let mut content_file = OpenOptions::new().append(true).create(true).open(format!("{}/content.bin", path.clone()))?;
        let default_chunk_size = 10 * 1024;
        let mut read_size: usize = 0;
        let data_size = metadata.size as usize;
        loop {
            // TODO: add timeout so stream wont be idle for too long
            let chunk_size = if data_size - read_size > default_chunk_size { default_chunk_size } else { data_size - read_size };
            if chunk_size == 0 {
                stream.write_all(&length_buf).await?;
                
                stream.write_all(&buffer).await?;
                stream.flush().await?;

                data_ids.push(data_id.clone());

                content_file.flush()?;
                println!("Stored data: {}, size: {}", metadata.name, metadata.size);
                // notify.send(path.clone()).await.expect("Failed to send notification");
                break;
            }
            let mut buffer = vec![0u8; chunk_size];
            stream.read_exact(&mut buffer).await?;

            read_size+=chunk_size;

            content_file.write_all(&buffer)?;
            println!("Received chunk: {}/{}", read_size, metadata.size);
        }
    };
}

async fn serve_data_v1(base_path: String, mut stream: Stream, mut c: Networking) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let header = handshake(&mut stream).await?;
    c.client.subscribe(header.job_id.clone()).await?;
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf).await?;
    let input = deserialize_from_slice::<ConsumeDataInputV1>(&buf)?;
    // let query = input.query.clone();
    let name_regexp = {
        let query = input.query.clone();
        if let Some(name_regexp) = query.name_regexp {
            name_regexp
        } else {
            ".*".to_string()
        }
    };
    let ids_filter ={
        let query = input.query.clone();
        query.ids.clone()
    };
    // let listener = listener.clone();
    let paths = std::fs::read_dir(format!("{}/output/domain_data", base_path))?;

    for path in paths {
        let path = path.expect("Failed to read path").path();
        let data_id = path.file_name().expect("Failed to get file name").to_str().expect("Failed to convert to str").to_string();
        let metadata_path = format!("{}/metadata.bin", path.to_str().expect("Failed to convert to str"));
        let content_path = format!("{}/content.bin", path.to_str().expect("Failed to convert to str"));

        let metadata_buf = std::fs::read(metadata_path)?;
        let metadata = deserialize_from_slice::<Metadata>(&metadata_buf)?;
        if !regex::Regex::new(&name_regexp).unwrap().is_match(metadata.name.as_str()) {
            continue;
        }
        if ids_filter.len() > 0 && !ids_filter.contains(&metadata.id.unwrap()) {
            continue;
        }
        let mut length_buf = [0u8; 4];
        let length = metadata_buf.len() as u32;
        length_buf.copy_from_slice(&length.to_be_bytes());
        stream.write_all(&length_buf).await?;
        stream.write_all(&metadata_buf).await?;
        
        let mut f = fs::File::open(content_path)?;
        let mut written = 0;
        let chunk_size = 2 * 1024;
        loop {
            let mut buf = vec![0; chunk_size];
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            written += n;
            println!("Served chunk: {}/{}", written, metadata.size);
            stream.write_all(&buf[..n]).await?;
            stream.flush().await?;
        }
        println!("Served data: {}, size: {}", metadata.name, metadata.size);
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
    let _ = std::fs::remove_dir_all(format!("{}/output/domain_data", base_path));
    std::fs::create_dir_all(format!("{}/output/domain_data", base_path)).expect("Failed to create domain_data directory");

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
            else => break
        }
    }

    Ok(())
}

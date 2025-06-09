use futures::{AsyncWrite, StreamExt};
use quick_protobuf::deserialize_from_slice;
use std::{collections::HashMap, fs, io::{Error, ErrorKind, Read}, pin::Pin, sync::{Arc, Mutex}, task::{Context, Poll}, vec};
use posemesh_domain::{cluster::{DomainCluster, TaskUpdateEvent, TaskUpdateResult}, datastore::{common::{data_id_generator, Datastore}, remote::RemoteDatastore}, protobuf::domain_data::{self, Metadata, Query, UpsertMetadata}, spatial::reconstruction::reconstruction_job};

/*
    * This is a client that wants to do reconstruction in domain cluster
    * Usage: cargo run --package client-example <port> <name> <domain_manager> <domain_id> <relay>
    * Example: cargo run --package client-example 0 dmt /ip4/1.2.3.4/udp/18804/quic-v1/p2p/12D3KooWBMyph6PCuP6GUJkwFdR7bLUPZ3exLvgEPpR93J52GaJg 12D3KooWBMyph6PCuP6GUJkwFdR7bLUPZ3exLvgEPpR93J52GaJg
*/
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        println!("Usage: {} <port> <name> <domain_manager> <domain_id> <relay>", args[0]);
        return Ok(());
    }
    let port = args[1].parse::<u16>().unwrap();
    let name = args[2].clone();
    let domain_manager = args[3].clone();
    let domain_id = args[4].clone();
    let relay = if args.len() > 5 {
        vec![args[5].clone()]
    } else {
        vec![]
    };
    let base_path = format!("./volume/{}", name);
    let private_key_path = format!("{}/pkey", base_path);

    let domain_cluster = DomainCluster::join(&domain_manager, &name, false, port, false, false, None, Some(private_key_path), relay).await.expect("failed to join cluster");
    let mut remote_datastore = RemoteDatastore::new(domain_cluster.clone());
    
    let input_dir = format!("{}/input", base_path);
    fs::create_dir_all(&input_dir).expect("cant create input dir");
    let dir = fs::read_dir(input_dir).unwrap();
    let scan = "2025-02-26_11-19-47".to_string();

    let query = Query {
        ids: vec![],
        names: vec![],
        data_types: vec![],
        name_regexp: Some(format!(".*_{}", scan)),
        data_type_regexp: None,
        metadata_only: true,
    };

    #[derive(Clone)]
    struct DataConsumer {
        name_to_id: Arc<Mutex<HashMap<String, String>>>,
    }
    impl AsyncWrite for DataConsumer {
        fn poll_write(self: Pin<&mut Self>, _: &mut Context<'_>, content: &[u8]) -> Poll<Result<usize, Error>> {
            if content.len() < 4 {
                return Poll::Ready(Err(Error::new(ErrorKind::UnexpectedEof, "Incomplete metadata")));
            }

            match deserialize_from_slice::<domain_data::Metadata>(&content[4..]) {
                Ok(metadata) => {
                    let mut name_to_id = self.name_to_id.lock().unwrap();
                    name_to_id.insert(metadata.name, metadata.id);
                    return Poll::Ready(Ok(content.len()));
                }
                Err(e) => {
                    return Poll::Ready(Err(Error::new(ErrorKind::Other, "Failed to deserialize metadata")));
                }
            }
        }
        
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }
        
        fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }
        
    }

    let name_to_id = Arc::new(Mutex::new(HashMap::new()));

    let writer = DataConsumer {
        name_to_id: name_to_id.clone(),
    };

    let mut downloader = remote_datastore.load(domain_id.clone(), query, false, writer).await?;
    downloader.wait_for_done().await?;
    println!("downloaded {} files", name_to_id.lock().unwrap().len());

    let mut producer = remote_datastore.upsert(domain_id.clone()).await?;

    for entry in dir {
        let entry = entry.unwrap();
        let path = entry.path().clone();

        if !path.is_file() {
            continue;
        }

        match fs::File::open(path.clone()) {
            Ok(mut f) => {
                if f.metadata()?.len() > u32::MAX as u64 {
                    println!("File too large: {:?}", f.metadata()?.len());
                    continue;
                }

                let file_name = entry.file_name().into_string().unwrap();
                let parts = file_name.split(".").collect::<Vec<&str>>();
                let data_type = parts.last().unwrap();
                let name = format!("{}_{}", parts[..parts.len()-1].join("."), scan);

                let id = name_to_id.lock().unwrap().get(&name).map(|id| id.clone());
                let metadata = UpsertMetadata {
                    id: id.clone().unwrap_or_else(|| data_id_generator()),
                    name,
                    data_type: data_type.to_string(),
                    size: f.metadata()?.len() as u32,
                    properties: HashMap::new(),
                    is_new: id.is_none(),
                };

                let mut content = vec![0u8; metadata.size as usize];
                f.read_exact(&mut content).expect("cant read file");

                let mut writer = producer.push(&metadata).await.expect("cant push data");
                writer.next_chunk(&content, false).await.expect("cant push chunk");
            }
            Err(e) => {
                println!("Error reading file: {:?}", e);
            }
        }
    }

    while !producer.is_completed().await {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
    producer.close().await;

    println!("producer closed");

    let mut recv = reconstruction_job(domain_cluster, &domain_id, vec![scan]).await; 

    loop {
        tokio::select! {
            Some(TaskUpdateEvent {
                result: TaskUpdateResult::Ok(task),
                ..
            }) = recv.next() => {
                println!("Received task {} status update: {:?}", task.name, task.status);
            }
            else => {
                break;
            }
        }
    }

    Ok(())
}

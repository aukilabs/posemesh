use domain::{cluster::DomainCluster, datastore::{common::Datastore, remote::RemoteDatastore}, message::read_prefix_size_message, protobuf::{domain_data::Query,task::{self, LocalRefinementInputV1, LocalRefinementOutputV1}}};
use jsonwebtoken::{decode, DecodingKey,Validation, Algorithm};
use libp2p::Stream;
use networking::{client::Client, libp2p::Networking};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use tokio::{self, select, time::{sleep, Duration}};
use futures::{AsyncReadExt, StreamExt};
use uuid::Uuid;
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

    decode_jwt(header.access_token.as_str())
}

async fn local_refinement_v1(mut stream: Stream, mut datastore: Box<dyn Datastore>, mut c: Client) {
    let claim = handshake(&mut stream).await.expect("Failed to handshake");
    let job_id = claim.job_id.clone();
    c.subscribe(job_id.clone()).await.expect("Failed to subscribe to job");
    let t = &mut task::Task {
        name: claim.task_name.clone(),
        receiver: claim.receiver.clone(),
        sender: claim.sender.clone(),
        endpoint: "/local-refinement/v1".to_string(),
        status: task::Status::STARTED,
        access_token: "".to_string(),
        job_id: job_id.clone(),
        output: None,
    };

    let res = read_prefix_size_message::<LocalRefinementInputV1>(stream).await;
    if let Err(e) = res {
        eprintln!("Failed to load local refinement input {}", e);
        t.status = task::Status::FAILED;
        let message = serialize_into_vec(t).expect("failed to serialize task update");
        c.publish(job_id.clone(), message).await.expect("failed to publish task update");
        return;
    }
    let input = res.unwrap();

    c.publish(job_id.clone(), serialize_into_vec(t).expect("failed to serialize task update")).await.expect("failed to publish task update");
    println!("Start executing {}, {:?}", claim.task_name, input);

    let mut downloader = datastore.consume("".to_string(), Query { ids: vec![Uuid::new_v4().to_string()], name_regexp: None, data_type_regexp: None, names: vec![], data_types: vec![] }, false).await;
    let mut i = 0;
    loop {
        match downloader.next().await {
            Some(Ok(_)) => {
                i+=1;
            }
            Some(Err(_)) => {
                t.status = task::Status::RETRY;
                let message = serialize_into_vec(t).expect("failed to serialize task update");
                c.publish(job_id.clone(), message).await.expect("failed to publish task update");
                return;
            }
            None => {
                break;
            }
        }
    }
    println!("Finished downloading {} data for {}", i, claim.task_name);

    let output = LocalRefinementOutputV1 {
        result_ids: vec![Uuid::new_v4().to_string()],
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
}

async fn global_refinement_v1(mut stream: Stream, mut c: Networking) {
    let claim = handshake(&mut stream).await.expect("Failed to handshake");
    let job_id = claim.job_id.clone();
    c.client.subscribe(job_id.clone()).await.expect("Failed to subscribe to job");
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
    c.client.publish(job_id.clone(), buf).await.expect("failed to publish task update");
    println!("Finished executing {}", claim.task_name);
}
/*
    * This is a simple example of a reconstruction node. It will connect to a set of bootstraps and execute reconstruction jobs.
    * Usage: cargo run --package reconstruction-node <port> <name> <domain_manager> 
    * Example: cargo run --package reconstruction-node 18808 reconstruction 
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
    let mut n = domain_cluster.peer.clone();
    let mut local_refinement_v1_handler = n.client.set_stream_handler("/local-refinement/v1".to_string()).await.unwrap();
    let mut global_refinement_v1_handler = n.client.set_stream_handler("/global-refinement/v1".to_string()).await.unwrap();
    let remote_storage = RemoteDatastore::new(domain_cluster);

    loop {
        select! {
            Some((_, stream)) = local_refinement_v1_handler.next() => {
                let _ = tokio::spawn(local_refinement_v1(stream, Box::new(remote_storage.clone()), n.client.clone()));
            }
            Some((_, stream)) = global_refinement_v1_handler.next() => {
                let _ = tokio::spawn(global_refinement_v1(stream, n.clone()));
            }
            else => break
        }
    }

    Ok(())
}

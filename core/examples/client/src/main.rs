use networking::{context, network};
use tokio;
use futures::StreamExt;
use std::{collections::HashMap, fs, io::Read, vec};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use domain::{cluster::{DomainCluster, TaskUpdateEvent, TaskUpdateResult}, datastore::{common::{data_id_generator, Datastore}, remote::RemoteDatastore}, protobuf::{domain_data::{Data, Metadata, Query}, task::{self, mod_ResourceRecruitment as ResourceRecruitment, Status}}};

const MAX_MESSAGE_SIZE_BYTES: usize = 1024 * 1024 * 10;

/*
    * This is a client that wants to do reconstruction in domain cluster
    * Usage: cargo run --example dmt --features rust <port> <name> <domain_manager>
    * Example: cargo run --example dmt --features rust 0 dmt /ip4/54.67.15.233/udp/18804/quic-v1/p2p/12D3KooWBMyph6PCuP6GUJkwFdR7bLUPZ3exLvgEPpR93J52GaJg
*/
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!("Usage: {} <port> <name> <bootstraps>", args[0]);
        return Ok(());
    }
    let port = args[1].parse::<u16>().unwrap();
    let name = args[2].clone();
    let domain_manager = args[3].clone();
    let base_path = format!("./volume/{}", name);
    let private_key_path = format!("{}/pkey", base_path);

    let cfg = &network::NetworkingConfig{
        port: port,
        bootstrap_nodes: vec![domain_manager.clone()],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![domain_manager.clone()],
        private_key: vec![],
        private_key_path: private_key_path,
        name: name,
    };
    let mut c = context::context_create(cfg)?;
    let peer_id = c.id.clone();
    let c_clone = c.clone();

    let domain_manager_id = domain_manager.split("/").last().unwrap().to_string();

    let mut domain_cluster = DomainCluster::new(domain_manager_id.clone(), Box::new(c_clone));
    let mut remote_datastore = RemoteDatastore::new(domain_cluster.clone(), c.clone());
    
    let input_dir = format!("{}/input", base_path);
    fs::create_dir_all(&input_dir).expect("cant create input dir");
    let dir = fs::read_dir(input_dir).unwrap();

    let mut producer = remote_datastore.produce(domain_manager_id.clone()).await;
    let mut uploaded = Vec::<task::TaskRequest>::new();

    for entry in dir {
        let entry = entry.unwrap();
        let path = entry.path().clone();

        match fs::File::open(path) {
            Ok(mut f) => {
                if f.metadata()?.len() > u32::MAX as u64 {
                    println!("File too large: {:?}", f.metadata()?.len());
                    continue;
                }

                let metadata = Metadata {
                    id: Some(data_id_generator()),
                    name: entry.file_name().to_string_lossy().to_string(),
                    data_type: "image".to_string(),
                    size: f.metadata()?.len() as u32,
                    properties: HashMap::new(),
                };

                let mut content = vec![0u8; metadata.size as usize];
                f.read_exact(&mut content).expect("cant read file");

                let data = Data {
                    domain_id: domain_manager_id.clone(),
                    metadata: metadata.clone(),
                    content
                };

                let id = producer.push(&data).await.expect("cant push data");

                let input = task::LocalRefinementInputV1 {
                    query: Some(Query {
                        ids: vec![id.clone()],
                        name_regexp: None,
                        data_type_regexp: None,
                        names: vec![],
                        data_types: vec![],
                    }),
                };
                let task = task::TaskRequest {
                    needs: vec![],
                    resource_recruitment: Some(task::ResourceRecruitment {
                        recruitment_policy: ResourceRecruitment::RecruitmentPolicy::ALWAYS,
                        termination_policy: ResourceRecruitment::TerminationPolicy::TERMINATE,
                    }),
                    name: format!("local_refinement_{}", id.clone()),
                    timeout: "10h".to_string(),
                    max_budget: 1000,
                    capability_filters: Some(task::CapabilityFilters {
                        endpoint: "/local-refinement/v1".to_string(),
                        min_gpu: 0,
                        min_cpu: 0,
                    }),
                    data: Some(task::Any {
                        type_url: "LocalRefinementInputV1".to_string(), // TODO: use actual type url
                        value: serialize_into_vec(&input).expect("cant serialize input"),
                    }),
                    sender: domain_manager_id.clone(),
                    receiver: "".to_string(),
                };
                uploaded.push(task);
            }
            Err(e) => {
                println!("Error reading file: {:?}", e);
            }
        }
    }
    
    // TODO: use oneshot channel to signal completion
    while !producer.is_completed().await {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
    producer.close().await;
    
    let dependencies = uploaded.iter().map(|t| t.name.clone()).collect::<Vec<String>>();
    uploaded.push(task::TaskRequest {
        needs: dependencies,
        resource_recruitment: Some(task::ResourceRecruitment {
            recruitment_policy: ResourceRecruitment::RecruitmentPolicy::ALWAYS,
            termination_policy: ResourceRecruitment::TerminationPolicy::KEEP,
        }),
        name: "global_refinement".to_string(),
        timeout: "10m".to_string(),
        max_budget: 1000,
        capability_filters: Some(task::CapabilityFilters {
            endpoint: "/global-refinement/v1".to_string(),
            min_gpu: 1,
            min_cpu: 1,
        }),
        data: Some(task::Any {
            type_url: "GlobalRefinementInputV1".to_string(), // TODO: use actual type url
            value: vec![],
        }),
        sender: domain_manager_id.clone(),
        receiver: "".to_string(),
    });

    let job = task::Job {
        name: "local_refinement".to_string(),
        tasks: uploaded,
    };

    let mut recv = domain_cluster.submit_job(&job).await;

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

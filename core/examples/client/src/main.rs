use networking::{context, network};
use tokio;
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use protobuf::{domain_data, task::{self, mod_ResourceRecruitment as ResourceRecruitment, Status}};
use std::{collections::HashMap, fs, io::Read, vec};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use domain::{cluster::{DomainCluster, TaskUpdateEvent, TaskUpdateResult}, datastore::remote::RemoteDatastore};

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
    
    let input_dir = format!("{}/input", base_path);
    fs::create_dir_all(&input_dir).expect("cant create input dir");
    let dir = fs::read_dir(input_dir).unwrap();

    let mut upload_job_recv = domain_cluster.submit_job(&task::Job {
        name: "stream uploading recordings".to_string(),
        tasks: vec![
            task::TaskRequest {
                needs: vec![],
                resource_recruitment: Some(task::ResourceRecruitment {
                    recruitment_policy: ResourceRecruitment::RecruitmentPolicy::FAIL,
                    termination_policy: ResourceRecruitment::TerminationPolicy::KEEP,
                }),
                name: "store_recording".to_string(),
                timeout: "100m".to_string(),
                max_budget: 1000,
                capability_filters: Some(task::CapabilityFilters {
                    endpoint: "/store/v1".to_string(),
                    min_gpu: 0,
                    min_cpu: 0,
                }),
                data: None,
                sender: peer_id.clone(),
                receiver: "".to_string(),
            }
        ],
    }).await;

    let mut upload_task: task::Task;
    loop {
        if let Ok(Some(TaskUpdateEvent {
            result: TaskUpdateResult::Ok(task),
            ..
        })) = upload_job_recv.try_next() {
            if task.status == Status::PENDING {
                upload_task = task.clone();
                break;
            }
        }
    }

    let m_buf = serialize_into_vec(&task::DomainClusterHandshake{
        access_token: upload_task.access_token.clone(),
    })?;
    let mut length_buf = [0u8; 4];
    let length = m_buf.len() as u32;
    length_buf.copy_from_slice(&length.to_be_bytes());
    let mut upload_stream = c.send(length_buf.to_vec(), upload_task.receiver.clone(), upload_task.endpoint.clone(), 1000).await.expect("cant send handshake");
    upload_stream.write_all(&m_buf).await.expect("cant write handshake");
    upload_stream.flush().await.expect("cant flush handshake");
    upload_task.status = Status::STARTED;
    c.publish(upload_task.job_id.clone(), serialize_into_vec(&upload_task.clone()).expect("cant serialize task update")).await.expect("cant publish task update");
    
    let mut uploading = 0;

    // TODO: put upload_stream reader into a task

    // let borrowed = upload_stream.borrow_mut();
    // let mut framed_io = Framed::new(
    //     borrowed,
    //     quick_protobuf_codec::Codec::<task::DomainDataMetadata>::new(MAX_MESSAGE_SIZE_BYTES),
    // );

    for entry in dir {
        let entry = entry.unwrap();
        let path = entry.path().clone();
        // let mut d = domain_cluster.clone();
        // let peer_id = c.id.clone();

        match fs::File::open(path) {
            Ok(mut f) => {
                let chunk_size = 2 * 1024;

                if f.metadata()?.len() > u32::MAX as u64 {
                    println!("File too large: {:?}", f.metadata()?.len());
                    continue;
                }

                let metadata = domain_data::DomainDataMetadata {
                    name: entry.file_name().to_string_lossy().to_string(),
                    data_type: "image".to_string(),
                    size: f.metadata()?.len() as u32,
                    properties: HashMap::new(),
                    hash: "somedata".to_string(),
                };

                // framed_io.send(metadata).await?;

                let m_buf = serialize_into_vec(&metadata)?;
                let mut length_buf = [0u8; 4];
                let length = m_buf.len() as u32;
                length_buf.copy_from_slice(&length.to_be_bytes());

                upload_stream.write_all(&length_buf).await.expect("cant write length");
                upload_stream.write_all(&m_buf).await.expect("cant write metadata");
                // upload_stream.flush().await.expect("cant flush metadata");
                let mut written = 0;
                loop {
                    let mut buf = vec![0; chunk_size];
                    let n = f.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    written += n;
                    println!("Wrote chunk: {}/{}", written, metadata.size);
                    upload_stream.write(&buf[..n]).await.expect("cant write chunk");
                    upload_stream.flush().await.expect("cant flush chunk");
                }
                uploading+=1;
                println!("Uploaded file: {:?}", metadata);
            }
            Err(e) => {
                println!("Error reading file: {:?}", e);
            }
        }
    }
    let mut uploaded = Vec::<task::TaskRequest>::with_capacity(uploading);
    for i in 0..uploading {
        // let info = framed_io
        // .next()
        // .await
        // .ok_or(UpgradeError::StreamClosed)??;
        let mut length_buf = [0u8; 4];
        upload_stream.read_exact(&mut length_buf).await?;

        let length = u32::from_be_bytes(length_buf) as usize;
        let mut buffer = vec![0u8; length];
        upload_stream.read_exact(&mut buffer).await?;
            
        let info = deserialize_from_slice::<domain_data::DomainDataMetadata>(&buffer)?;

        let input = task::LocalRefinementInputV1 {
            recording_id: info.hash.clone(),
        };
        let task = task::TaskRequest {
            needs: vec![],
            resource_recruitment: Some(task::ResourceRecruitment {
                recruitment_policy: ResourceRecruitment::RecruitmentPolicy::ALWAYS,
                termination_policy: ResourceRecruitment::TerminationPolicy::TERMINATE,
            }),
            name: format!("local_refinement_{}", i),
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
    // framed_io.close().await?; // Must close the stream to prevent memory leaks
    upload_stream.close().await.expect("cant close stream");
    let task_update = task::Task {
        name: upload_task.name.clone(),
        receiver: upload_task.sender.clone(),
        sender: upload_task.receiver.clone(),
        endpoint: upload_task.endpoint.clone(),
        status: Status::DONE,
        access_token: upload_task.access_token.clone(),
        job_id: upload_task.job_id.clone(),
        output: None,
    };
    c.publish(upload_task.job_id.clone(), serialize_into_vec(&task_update).expect("cant serialize task update")).await.expect("cant publish store data task update");

    let dependencies = uploaded.iter().map(|t| t.name.clone()).collect::<Vec<String>>();
    // let output = uploaded.iter().map(|t| LocalRefinementOutputV1 {
    //     recording_id: format!("${{tasks.{}.outputs.recording_id}}", t.name.clone()),
    //     result_id: format!("${{tasks.{}.outputs.result_id}}", t.name.clone()),
    // }).collect::<Vec<task::LocalRefinementOutputV1>>();
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

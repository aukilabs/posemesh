use futures::channel::mpsc::Receiver;
use quick_protobuf::serialize_into_vec;

use crate::{cluster::{DomainCluster, TaskUpdateEvent}, protobuf::{domain_data::Query, task}};

pub async fn reconstruction_job(mut domain_cluster: DomainCluster, domain_id: &str, scans: Vec<String>) -> Receiver<TaskUpdateEvent> {
    let mut uploaded = Vec::<task::TaskRequest>::new();
    for scan in scans {
        let input = task::LocalRefinementInputV1 {
            query: Query {
                ids: vec![],
                name_regexp: Some(format!(".*_{}", scan)),
                data_type_regexp: None,
                names: vec![],
                data_types: vec![],
            },
        };
        let task = task::TaskRequest {
            needs: vec![],
            resource_recruitment: task::ResourceRecruitment {
                recruitment_policy: task::mod_ResourceRecruitment::RecruitmentPolicy::ALWAYS,
                termination_policy: task::mod_ResourceRecruitment::TerminationPolicy::TERMINATE,
            },
            name: format!("local_refinement_{}", scan),
            timeout: "10h".to_string(),
            max_budget: Some(1000),
            capability_filters: task::CapabilityFilters {
                endpoint: "/local-refinement/v1".to_string(),
                min_gpu: Some(0),
                min_cpu: Some(0),
            },
            data: Some(task::Any {
                type_url: "LocalRefinementInputV1".to_string(), // TODO: use actual type url
                value: serialize_into_vec(&input).expect("cant serialize input"),
            }),
            sender: domain_cluster.manager_id.clone(),
            receiver: None,
        };
        uploaded.push(task);
    }

    let dependencies = uploaded.iter().map(|t| t.name.clone()).collect::<Vec<String>>();
    uploaded.push(task::TaskRequest {
        needs: dependencies,
        resource_recruitment: task::ResourceRecruitment {
            recruitment_policy: task::mod_ResourceRecruitment::RecruitmentPolicy::ALWAYS,
            termination_policy: task::mod_ResourceRecruitment::TerminationPolicy::KEEP,
        },
        name: "global_refinement".to_string(),
        timeout: "10m".to_string(),
        max_budget: Some(1000),
        capability_filters: task::CapabilityFilters {
            endpoint: "/global-refinement/v1".to_string(),
            min_gpu: Some(1),
            min_cpu: Some(1),
        },
        data: Some(task::Any {
            type_url: "GlobalRefinementInputV1".to_string(), // TODO: use actual type url
            value: vec![],
        }),
        sender: domain_cluster.manager_id.clone(),
        receiver: None,
    });

    let job = task::JobRequest {
        domain_id: domain_id.to_string(),
        name: "refinement job".to_string(),
        tasks: uploaded,
        nonce: "".to_string(),
    };

    tracing::debug!("job has {} tasks", job.tasks.len());

    domain_cluster.submit_job(&job).await
}

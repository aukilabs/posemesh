use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use reqwest::{redirect::Policy, Client, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::time::{sleep, Instant};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use url::Url;
use uuid::Uuid;

use crate::config::{JobConfig, TaskMode};

pub const ROBOT_CAPABILITY: &str = "/dmtbot/scan-path/v0";
pub const RECONSTRUCTION_CAPABILITY: &str = "/reconstruction/local-refinement-auki-sdk/v0";
const NO_NODES_PREFIX: &str = "no nodes available for capability";
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

#[async_trait]
pub trait ReconstructionJobSubmitter: Send + Sync {
    async fn submit_reconstruction(
        &self,
        run_id: Uuid,
        reference_artifact_id: &str,
    ) -> Result<Uuid>;
}

#[derive(Clone)]
pub struct JobClient {
    http: Client,
    jobs_url: Url,
    config: JobConfig,
}

impl JobClient {
    pub fn new(config: JobConfig) -> Result<Self> {
        let mut jobs_url = config.dms_base_url.clone();
        jobs_url
            .path_segments_mut()
            .map_err(|_| anyhow!("DMS_BASE_URL cannot be used as a base URL"))?
            .pop_if_empty()
            .push("jobs");
        let http = Client::builder()
            .use_rustls_tls()
            .redirect(Policy::none())
            .timeout(HTTP_TIMEOUT)
            .build()
            .context("build relay demo DMS client")?;
        Ok(Self {
            http,
            jobs_url,
            config,
        })
    }

    pub async fn submit_robot(&self, run_id: Uuid, shutdown: &CancellationToken) -> Result<Uuid> {
        let payload = robot_job_payload(self.config.domain_id, run_id);
        self.create_when_available(&payload, shutdown).await
    }

    async fn create_when_available(
        &self,
        payload: &Value,
        shutdown: &CancellationToken,
    ) -> Result<Uuid> {
        let deadline = Instant::now() + self.config.node_wait;
        loop {
            if Instant::now() >= deadline {
                bail!(
                    "timed out after {:?} waiting for an eligible dev node",
                    self.config.node_wait
                );
            }

            let request = self
                .http
                .post(self.jobs_url.clone())
                .bearer_auth(self.config.app_jwt.as_ref())
                .json(payload)
                .send();
            let response = tokio::select! {
                result = request => result.context("send DMS demo job request")?,
                _ = shutdown.cancelled() => bail!("demo job submission canceled"),
            };
            let status = response.status();
            if response
                .content_length()
                .is_some_and(|size| size > u64::try_from(MAX_RESPONSE_BYTES).unwrap_or(u64::MAX))
            {
                bail!("DMS demo job response exceeded its size limit");
            }
            let body = response
                .bytes()
                .await
                .context("read DMS demo job response")?;
            if body.len() > MAX_RESPONSE_BYTES {
                bail!("DMS demo job response exceeded its size limit");
            }

            if status.is_success() {
                let created: CreateJobResponse =
                    serde_json::from_slice(&body).context("decode DMS demo job response")?;
                info!(job_id = %created.job_id, "dev relay demo job created");
                return Ok(created.job_id);
            }

            let no_nodes = status == StatusCode::BAD_REQUEST
                && serde_json::from_slice::<ErrorResponse>(&body)
                    .ok()
                    .is_some_and(|error| is_no_nodes_error(&error.error));
            if !no_nodes {
                bail!("DMS demo job request returned HTTP {status}");
            }

            warn!("waiting for the matching dev runner to become visible in DMS");
            let remaining = deadline.saturating_duration_since(Instant::now());
            let delay = RETRY_DELAY.min(remaining);
            tokio::select! {
                _ = sleep(delay) => {}
                _ = shutdown.cancelled() => bail!("demo job submission canceled"),
            }
        }
    }
}

fn is_no_nodes_error(message: &str) -> bool {
    message
        .strip_prefix("validation error: ")
        .unwrap_or(message)
        .starts_with(NO_NODES_PREFIX)
}

#[async_trait]
impl ReconstructionJobSubmitter for JobClient {
    async fn submit_reconstruction(
        &self,
        run_id: Uuid,
        reference_artifact_id: &str,
    ) -> Result<Uuid> {
        if reference_artifact_id.trim().is_empty() {
            bail!("reference artifact ID is empty");
        }
        let payload = reconstruction_job_payload(
            self.config.domain_id,
            run_id,
            reference_artifact_id,
            self.config.reconstruction_mode,
        );
        self.create_when_available(&payload, &CancellationToken::new())
            .await
    }
}

pub fn robot_job_payload(domain_id: Uuid, run_id: Uuid) -> Value {
    json!({
        "label": format!("relay-file-demo-publish-{run_id}"),
        "domain_id": domain_id,
        "priority": 0,
        "meta": {
            "relay_file_demo": true,
            "run_id": run_id,
        },
        "tasks": [{
            "label": "relay-file-demo-publish",
            "stage": "publish",
            "capability": ROBOT_CAPABILITY,
            "capability_filters": {},
            "priority": 0,
            "inputs_cids": [],
            "outputs_prefix": format!("relay-file-demo/{run_id}/robot/"),
            "mode": "dedicated",
            "meta": {
                "relay_file_demo": true,
                "run_id": run_id,
            },
            "max_attempts": 1,
        }],
        "edges": [],
    })
}

pub fn reconstruction_job_payload(
    domain_id: Uuid,
    run_id: Uuid,
    reference_artifact_id: &str,
    mode: TaskMode,
) -> Value {
    json!({
        "label": format!("relay-file-demo-download-{run_id}"),
        "domain_id": domain_id,
        "priority": 0,
        "meta": {
            "relay_file_demo": true,
            "run_id": run_id,
        },
        "tasks": [{
            "label": "relay-file-demo-download",
            "stage": "download",
            "capability": RECONSTRUCTION_CAPABILITY,
            "capability_filters": {},
            "priority": 0,
            "inputs_cids": [reference_artifact_id],
            "outputs_prefix": format!("relay-file-demo/{run_id}/reconstruction/"),
            "mode": mode.as_str(),
            "meta": {
                "relay_file_demo": true,
                "run_id": run_id,
            },
            "max_attempts": 1,
        }],
        "edges": [],
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateJobResponse {
    job_id: Uuid,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::{Method::POST, MockServer};
    use std::sync::Arc;

    #[test]
    fn robot_payload_is_one_exact_dedicated_task() {
        let domain_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let payload = robot_job_payload(domain_id, run_id);
        assert_eq!(payload["domain_id"], domain_id.to_string());
        assert_eq!(payload["tasks"].as_array().unwrap().len(), 1);
        assert_eq!(payload["tasks"][0]["capability"], ROBOT_CAPABILITY);
        assert_eq!(payload["tasks"][0]["mode"], "dedicated");
        assert_eq!(payload["tasks"][0]["meta"]["run_id"], run_id.to_string());
    }

    #[test]
    fn reconstruction_payload_carries_only_the_reference_artifact() {
        let domain_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let payload =
            reconstruction_job_payload(domain_id, run_id, "reference-artifact", TaskMode::Public);
        assert_eq!(payload["tasks"][0]["capability"], RECONSTRUCTION_CAPABILITY);
        assert_eq!(payload["tasks"][0]["mode"], "public");
        assert_eq!(
            payload["tasks"][0]["inputs_cids"],
            json!(["reference-artifact"])
        );
        assert_eq!(payload["tasks"][0]["meta"]["run_id"], run_id.to_string());
    }

    #[test]
    fn dms_no_nodes_validation_is_a_safe_retry_signal() {
        assert!(is_no_nodes_error(
            "validation error: no nodes available for capability: /dmtbot/scan-path/v0 (mode dedicated)"
        ));
        assert!(is_no_nodes_error(
            "no nodes available for capability: /dmtbot/scan-path/v0"
        ));
        assert!(!is_no_nodes_error("validation error: invalid job payload"));
    }

    #[tokio::test]
    async fn job_client_posts_exact_bearer_and_decodes_job_id() {
        let server = MockServer::start_async().await;
        let domain_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let job_id = Uuid::new_v4();
        let payload = robot_job_payload(domain_id, run_id);
        let request = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/v1/jobs")
                    .header("authorization", "Bearer dev-app-token")
                    .json_body(payload);
                then.status(200).json_body(json!({"job_id": job_id}));
            })
            .await;
        let client = JobClient::new(JobConfig {
            dms_base_url: Url::parse(&server.url("/v1")).unwrap(),
            app_jwt: Arc::from("dev-app-token"),
            domain_id,
            reconstruction_mode: TaskMode::Dedicated,
            node_wait: Duration::from_secs(5),
        })
        .unwrap();

        let created = client
            .submit_robot(run_id, &CancellationToken::new())
            .await
            .unwrap();

        assert_eq!(created, job_id);
        request.assert_async().await;
    }

    #[tokio::test]
    async fn job_client_never_includes_a_reflected_bearer_in_errors() {
        let server = MockServer::start_async().await;
        let request = server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/jobs");
                then.status(403).json_body(json!({
                    "code": "forbidden",
                    "error": "reflected dev-app-token",
                }));
            })
            .await;
        let client = JobClient::new(JobConfig {
            dms_base_url: Url::parse(&server.url("/v1")).unwrap(),
            app_jwt: Arc::from("dev-app-token"),
            domain_id: Uuid::new_v4(),
            reconstruction_mode: TaskMode::Dedicated,
            node_wait: Duration::from_secs(5),
        })
        .unwrap();

        let error = client
            .submit_robot(Uuid::new_v4(), &CancellationToken::new())
            .await
            .unwrap_err()
            .to_string();

        request.assert_async().await;
        assert!(!error.contains("dev-app-token"));
        assert!(error.contains("HTTP 403"));
    }
}

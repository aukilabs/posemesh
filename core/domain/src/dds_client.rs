//! A client for talking to the Domain Discovery Service (DDS).
//!
//! This module provides:
//! 1) A clonable `DdsClient` you can share across tasks.
//! 2) An async `register` method with exponential backoff.
//! 3) A `shutdown` method to tell DDS you’re going away.
//!
//! Uses:
//! - `Arc<RwLock<…>>` for shared mutable state.
//! - `tokio::async` + `reqwest` for nonblocking HTTP.
//! - `thiserror` + `serde` for errors & (de)serialization.

use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use chrono::Utc;
use hex::encode as hex_encode;
use k256::{
    ecdsa::{signature::Signer, SigningKey},
    elliptic_curve::sec1::ToEncodedPoint,
    PublicKey,
};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::sleep;

// -------------------------------------
// 1) Errors
// -------------------------------------

/// Errors while talking to DDS.
#[derive(Debug, Error)]
pub enum DdsError {
    /// Any reqwest/http error.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    /// Non-2xx status from DDS.
    #[error("server returned {0}, body: {1}")]
    BadStatus(StatusCode, String),

    /// JSON (de)serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

// -------------------------------------
// 2) Payload types
// -------------------------------------

/// What we send to DDS.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DomainServerIn {
    url: String,
    registration_credentials: String,
    version: String,
    signature: String,
    timestamp: u64,
    public_key: String,
}

/// What DDS returns on register.
#[derive(Deserialize)]
struct RegisterResp {
    secret: String,
}

/// Credentials struct, used by clients and DDS shutdown.
#[derive(Serialize, Deserialize, Clone)]
pub struct DomainServerCredentials {
    pub id: String,
    pub secret: String,
}

// -------------------------------------
// 3) Internal status
// -------------------------------------

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Status {
    Disconnected,
    Registering,
    Registered,
}

// -------------------------------------
// 4) The client
// -------------------------------------

/// Cloneable DDS client.
#[derive(Clone)]
pub struct DdsClient {
    base_url: String,
    client: Client,
    domain_server_id: String,
    registration_credentials: String,
    domain_server_secret: Arc<RwLock<Option<String>>>,
    domain_server_version: String,
    last_healthcheck: Arc<RwLock<Instant>>,
    status: Arc<RwLock<Status>>,
}

impl DdsClient {
    /// Create a new client. `registration_credentials` must be "ID:SECRET".
    pub fn new(
        dds_endpoint: impl Into<String>,
        registration_credentials: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        let creds = registration_credentials.into();
        let domain_server_id = creds
            .splitn(2, ':')
            .next()
            .unwrap_or_default()
            .to_string();

        DdsClient {
            base_url: dds_endpoint.into(),
            client: Client::new(),
            domain_server_id,
            registration_credentials: creds,
            domain_server_secret: Arc::new(RwLock::new(None)),
            domain_server_version: version.into(),
            last_healthcheck: Arc::new(RwLock::new(Instant::now())),
            status: Arc::new(RwLock::new(Status::Disconnected)),
        }
    }

    /// Returns `true` once successfully registered.
    pub fn is_registered(&self) -> bool {
        *self.status.read().unwrap() == Status::Registered
    }

    /// Human-readable status.
    pub fn status(&self) -> String {
        format!("{:?}", *self.status.read().unwrap())
    }

    /// If registered, returns `{ id, secret }`.
    pub fn credentials(&self) -> Option<DomainServerCredentials> {
        self.domain_server_secret
            .read()
            .unwrap()
            .as_ref()
            .map(|secret| DomainServerCredentials {
                id: self.domain_server_id.clone(),
                secret: secret.clone(),
            })
    }

    /// Register (with exponential backoff) and then do health-checks forever.
    pub async fn register(
        &self,
        public_url: &str,
        healthcheck_ttl: Duration,
        max_retry: usize,
        signing_key: &SigningKey,
    ) -> Result<(), DdsError> {
        let mut attempt = 0;
        let mut backoff = Duration::from_secs(1);

        loop {
            // mark registering
            *self.status.write().unwrap() = Status::Registering;

            // build & sign payload
            let ts = Utc::now().timestamp() as u64;
            let msg = format!("{}:{}", public_url, ts);
            let sig: k256::ecdsa::Signature = signing_key.sign(msg.as_bytes());
            let pubkey = hex_encode(
                PublicKey::from(signing_key.verifying_key())
                    .to_encoded_point(false)
                    .as_bytes(),
            );

            let payload = DomainServerIn {
                url: public_url.to_string(),
                registration_credentials: self.registration_credentials.clone(),
                version: self.domain_server_version.clone(),
                signature: hex_encode(sig.as_ref()),
                timestamp: ts,
                public_key: pubkey,
            };

            let resp = self
                .client
                .post(format!("{}/internal/v1/register", self.base_url))
                .json(&payload)
                .send()
                .await?;

            if resp.status().is_success() {
                let body: RegisterResp = resp.json().await?;
                *self.domain_server_secret.write().unwrap() = Some(body.secret);
                *self.status.write().unwrap() = Status::Registered;
                *self.last_healthcheck.write().unwrap() = Instant::now();

                // sleep until next health-check
                sleep(healthcheck_ttl).await;
                attempt = 0;
                backoff = Duration::from_secs(1);
                continue;
            }

            // on error status
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if attempt < max_retry {
                attempt += 1;
                sleep(backoff).await;
                backoff = std::cmp::min(backoff * 2, Duration::from_secs(60));
                continue;
            } else {
                return Err(DdsError::BadStatus(status, text));
            }
        }
    }

    /// Notify DDS we’re shutting down (if we’re Registered).
    pub async fn shutdown(&self) -> Result<(), DdsError> {
        if let Some(creds) = self.credentials() {
            let resp = self
                .client
                .post(format!("{}/internal/v1/shutdown", self.base_url))
                .json(&creds)
                .send()
                .await?;

            if resp.status().is_success() {
                Ok(())
            } else {
                let st = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Err(DdsError::BadStatus(st, body))
            }
        } else {
            Ok(())
        }
    }
}

// -------------------------------------
// 5) Tests
// -------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};
    use serde_json::json;
    use rand::thread_rng;
    use k256::ecdsa::SigningKey;
    use reqwest::StatusCode;
    use std::time::Duration;
    use tokio::time::sleep;

    #[test]
    fn new_client_initial_state() {
        let client = DdsClient::new("http://example.com", "foo:bar", "1.0");
        assert!(!client.is_registered());
        assert_eq!(client.status(), "Disconnected");
        assert!(client.credentials().is_none());
    }

    #[tokio::test]
    async fn register_success_sets_status_and_secret() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/internal/v1/register"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "secret": "s1" })),
            )
            .mount(&server)
            .await;

        let client = DdsClient::new(&server.uri(), "id:cred", "v");
        let key = SigningKey::random(&mut thread_rng());
        let c2 = client.clone();
        let handle = tokio::spawn(async move {
            let _ = c2.register("http://callback", Duration::from_secs(3600), 3, &key).await;
        });

        for _ in 0..20 {
            if client.is_registered() {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
        assert!(client.is_registered());
        let creds = client.credentials().unwrap();
        assert_eq!(creds.id, "id");
        assert_eq!(creds.secret, "s1");
        handle.abort();
    }

    #[tokio::test]
    async fn register_returns_err_on_server_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/internal/v1/register"))
            .respond_with(
                ResponseTemplate::new(500).set_body_string("fail"),
            )
            .mount(&server)
            .await;

        let client = DdsClient::new(&server.uri(), "id:secret", "v");
        let key = SigningKey::random(&mut thread_rng());
        let err = client
            .register("http://callback", Duration::from_secs(1), 1, &key)
            .await
            .unwrap_err();
        match err {
            DdsError::BadStatus(status, body) => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert!(body.contains("fail"));
            }
            _ => panic!("expected BadStatus"),
        }
    }

    #[tokio::test]
    async fn shutdown_succeeds_when_registered() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/internal/v1/shutdown"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let client = DdsClient::new(&server.uri(), "id:secret", "ver");
        *client.status.write().unwrap() = Status::Registered;
        *client.domain_server_secret.write().unwrap() = Some("secret".to_string());

        assert!(client.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn shutdown_errors_on_non_ok() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/internal/v1/shutdown"))
            .respond_with(ResponseTemplate::new(500).set_body_string("oops"))
            .mount(&server)
            .await;

        let client = DdsClient::new(&server.uri(), "id:secret", "ver");
        *client.status.write().unwrap() = Status::Registered;
        *client.domain_server_secret.write().unwrap() = Some("secret".to_string());

        let err = client.shutdown().await.unwrap_err();
        match err {
            DdsError::BadStatus(code, body) => {
                assert_eq!(code, StatusCode::INTERNAL_SERVER_ERROR);
                assert!(body.contains("oops"));
            }
            _ => panic!("expected BadStatus"),
        }
    }
}

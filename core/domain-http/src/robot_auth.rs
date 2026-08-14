use base64::{Engine as _, engine::general_purpose};
use futures::lock::Mutex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth::{TokenCache, get_cached_or_fresh_token, parse_jwt};
use crate::discovery::{DomainServer, DomainWithServer, DomainWithToken};
use crate::errors::{AukiErrorResponse, AuthError, DomainError};

const VERIFY_PATH: &str = "/internal/v1/auth/robot/verify";
const DOMAIN_TOKEN_PATH: &str = "/internal/v1/auth/robot/domain-token";

#[derive(Debug, Clone)]
pub struct RobotAuthClient {
    dds_url: String,
    client_id: String,
    client: Client,
    registration_credentials: Option<String>,
    robot_token_cache: Arc<Mutex<Option<RobotTokenCache>>>,
    domain_token_cache: Arc<Mutex<Option<DomainWithToken>>>,
    session: Arc<Mutex<Option<RobotSession>>>,
}

#[derive(Debug, Clone)]
pub struct RobotSession {
    pub robot_id: String,
    pub assigned_domain_id: Option<String>,
}

#[derive(Debug, Clone)]
struct RobotTokenCache {
    access_token: String,
    expires_at: u64,
}

impl TokenCache for RobotTokenCache {
    fn get_access_token(&self) -> String {
        self.access_token.clone()
    }

    fn get_expires_at(&self) -> u64 {
        self.expires_at
    }
}

#[derive(Debug, Serialize)]
struct VerifyRequest<'a> {
    registration_credentials: &'a str,
}

#[derive(Debug, Deserialize)]
struct VerifyResponse {
    robot_id: String,
    access_token: String,
    #[serde(default)]
    access_expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct DomainTokenRequest<'a> {
    domain_id: &'a str,
}

#[derive(Debug, Deserialize)]
struct DomainTokenResponse {
    domain_id: String,
    domain_server_url: String,
    access_token: String,
    #[serde(default)]
    access_expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RobotAccessClaims {
    exp: u64,
    #[serde(default)]
    assigned_domain_id: Option<String>,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    sub: Option<String>,
}

impl RobotAuthClient {
    pub fn new(dds_url: &str, client_id: &str) -> Self {
        Self {
            dds_url: dds_url.trim_end_matches('/').to_string(),
            client_id: client_id.to_string(),
            client: Client::new(),
            registration_credentials: None,
            robot_token_cache: Arc::new(Mutex::new(None)),
            domain_token_cache: Arc::new(Mutex::new(None)),
            session: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn sign_in(
        &mut self,
        registration_credentials: &str,
    ) -> Result<RobotSession, DomainError> {
        let credentials = registration_credentials.trim();
        if credentials.is_empty() {
            return Err(DomainError::InvalidRequest(
                "registration_credentials must not be empty",
            ));
        }

        self.registration_credentials = Some(credentials.to_string());
        *self.robot_token_cache.lock().await = None;
        *self.domain_token_cache.lock().await = None;
        *self.session.lock().await = None;

        let token_cache = self.verify_credentials(credentials).await?;
        let claims = parse_robot_access_claims(&token_cache.access_token)?;
        let robot_id = claims
            .node_id
            .or(claims.sub)
            .filter(|id| !id.is_empty())
            .unwrap_or_default();
        if robot_id.is_empty() {
            return Err(AuthError::Unauthorized("robot token missing node_id").into());
        }

        let session = RobotSession {
            robot_id,
            assigned_domain_id: claims.assigned_domain_id.filter(|id| !id.is_empty()),
        };
        *self.robot_token_cache.lock().await = Some(token_cache);
        *self.session.lock().await = Some(session.clone());
        Ok(session)
    }

    pub async fn session(&self) -> Option<RobotSession> {
        self.session.lock().await.clone()
    }

    pub async fn assigned_domain_id(&self) -> Option<String> {
        self.session.lock().await.as_ref()?.assigned_domain_id.clone()
    }

    pub async fn robot_id(&self) -> Option<String> {
        Some(self.session.lock().await.as_ref()?.robot_id.clone())
    }

    pub async fn get_robot_access_token(&self) -> Result<String, DomainError> {
        let credentials = self.registration_credentials.as_ref().ok_or(
            AuthError::Unauthorized("robot registration credentials are not set"),
        )?;
        let cached = self.robot_token_cache.lock().await.clone().unwrap_or(RobotTokenCache {
            access_token: String::new(),
            expires_at: 0,
        });

        let fresh = get_cached_or_fresh_token(&cached, || {
            let credentials = credentials.clone();
            async move { self.verify_credentials(&credentials).await }
        })
        .await?;

        {
            let mut cache = self.robot_token_cache.lock().await;
            *cache = Some(fresh.clone());
        }

        // Keep session assignment in sync with refreshed JWT claims.
        if let Ok(claims) = parse_robot_access_claims(&fresh.access_token) {
            let mut session = self.session.lock().await;
            if let Some(existing) = session.as_mut() {
                existing.assigned_domain_id = claims.assigned_domain_id.filter(|id| !id.is_empty());
            }
        }

        Ok(fresh.access_token)
    }

    pub async fn auth_bound_domain(&self, domain_id: &str) -> Result<DomainWithToken, DomainError> {
        if domain_id.trim().is_empty() {
            return Err(DomainError::InvalidRequest("domain_id is required"));
        }

        let cached = self
            .domain_token_cache
            .lock()
            .await
            .clone()
            .filter(|cached| cached.domain.id == domain_id)
            .unwrap_or_else(|| {
                DomainWithToken::new(
                    DomainWithServer {
                        id: domain_id.to_string(),
                        name: String::new(),
                        organization_id: String::new(),
                        domain_server_id: String::new(),
                        redirect_url: None,
                        domain_server: DomainServer {
                            id: String::new(),
                            organization_id: String::new(),
                            name: String::new(),
                            url: String::new(),
                        },
                    },
                    String::new(),
                    0,
                )
            });

        let fresh = get_cached_or_fresh_token(&cached, || {
            let domain_id = domain_id.to_string();
            async move { self.exchange_domain_token(&domain_id).await }
        })
        .await?;

        {
            let mut cache = self.domain_token_cache.lock().await;
            *cache = Some(fresh.clone());
        }
        Ok(fresh)
    }

    async fn verify_credentials(
        &self,
        registration_credentials: &str,
    ) -> Result<RobotTokenCache, DomainError> {
        let response = self
            .client
            .post(format!("{}{}", self.dds_url, VERIFY_PATH))
            .header("Content-Type", "application/json")
            .header("posemesh-client-id", &self.client_id)
            .header("posemesh-sdk-version", crate::VERSION)
            .json(&VerifyRequest {
                registration_credentials,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AukiErrorResponse {
                status,
                error: format!("Failed to verify robot credentials. {}", text),
            }
            .into());
        }

        let body: VerifyResponse = response.json().await?;
        if body.access_token.is_empty() {
            return Err(AuthError::Unauthorized("robot verify returned empty access_token").into());
        }
        let _ = body.access_expires_at;
        let claims = parse_robot_access_claims(&body.access_token)?;
        let robot_id = claims
            .node_id
            .or(claims.sub)
            .unwrap_or_else(|| body.robot_id.clone());
        if robot_id.is_empty() {
            return Err(AuthError::Unauthorized("robot verify returned empty robot_id").into());
        }

        Ok(RobotTokenCache {
            access_token: body.access_token,
            expires_at: claims.exp,
        })
    }

    async fn exchange_domain_token(&self, domain_id: &str) -> Result<DomainWithToken, DomainError> {
        let robot_token = self.get_robot_access_token().await?;
        let response = self
            .client
            .post(format!("{}{}", self.dds_url, DOMAIN_TOKEN_PATH))
            .bearer_auth(robot_token)
            .header("Content-Type", "application/json")
            .header("posemesh-client-id", &self.client_id)
            .header("posemesh-sdk-version", crate::VERSION)
            .json(&DomainTokenRequest { domain_id })
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AukiErrorResponse {
                status,
                error: format!("Failed to exchange robot domain token. {}", text),
            }
            .into());
        }

        let body: DomainTokenResponse = response.json().await?;
        if body.access_token.is_empty() {
            return Err(AuthError::Unauthorized("domain-token returned empty access_token").into());
        }
        if body.domain_server_url.trim().is_empty() {
            return Err(AuthError::Unauthorized("domain-token returned empty domain_server_url").into());
        }
        let _ = body.access_expires_at;
        let expires_at = parse_jwt(&body.access_token)?.exp;

        Ok(DomainWithToken::new(
            DomainWithServer {
                id: body.domain_id,
                name: String::new(),
                organization_id: String::new(),
                domain_server_id: String::new(),
                redirect_url: None,
                domain_server: DomainServer {
                    id: String::new(),
                    organization_id: String::new(),
                    name: String::new(),
                    url: body.domain_server_url.trim_end_matches('/').to_string(),
                },
            },
            body.access_token,
            expires_at,
        ))
    }
}

fn parse_robot_access_claims(token: &str) -> Result<RobotAccessClaims, AuthError> {
    let parts = token.split('.').collect::<Vec<&str>>();
    if parts.len() != 3 {
        return Err(AuthError::Unauthorized("Invalid JWT token"));
    }
    let decoded = general_purpose::URL_SAFE_NO_PAD.decode(parts[1])?;
    let claims: RobotAccessClaims = serde_json::from_slice(&decoded)?;
    Ok(claims)
}

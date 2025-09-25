use std::{collections::HashMap, sync::Arc, time::Duration};

use futures::lock::Mutex;
use reqwest::Client;
use serde::Deserialize;

#[cfg(not(target_family = "wasm"))]
use tokio::spawn;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

use posemesh_utils::now_unix_secs;
#[cfg(target_family = "wasm")]
use posemesh_utils::sleep;
#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;

use crate::auth::{AuthClient, TokenCache, get_cached_or_fresh_token, parse_jwt};
pub const ALL_DOMAINS_ORG: &str = "all";
pub const OWN_DOMAINS_ORG: &str = "own";

#[derive(Debug, Deserialize, Clone)]
pub struct Domain {
    pub id: String,
    pub name: String,
    pub organization_id: String,
    pub domain_server_id: String,
    pub redirect_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DomainServer {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DomainWithToken {
    #[serde(flatten)]
    pub domain: DomainWithServer,
    #[serde(skip)]
    pub expires_at: u64,
    access_token: String,
}
impl TokenCache for DomainWithToken {
    fn get_access_token(&self) -> String {
        self.access_token.clone()
    }

    fn get_expires_at(&self) -> u64 {
        self.expires_at
    }

    fn set_expires_at(&mut self, expires_at: u64) {
        self.expires_at = expires_at;
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DomainWithServer {
    #[serde(flatten)]
    pub domain: Domain,
    pub domain_server: DomainServer,
}

#[derive(Debug, Clone)]
pub struct DiscoveryService {
    dds_url: String,
    client: Client,
    cache: Arc<Mutex<HashMap<String, DomainWithToken>>>,
    api_client: AuthClient,
    zitadel_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListDomainsResponse {
    pub domains: Vec<DomainWithServer>,
}

impl DiscoveryService {
    pub fn new(api_url: &str, dds_url: &str, client_id: &str) -> Self {
        let api_client = AuthClient::new(api_url, client_id);

        Self {
            dds_url: dds_url.to_string(),
            client: Client::new(),
            cache: Arc::new(Mutex::new(HashMap::new())),
            api_client,
            zitadel_token: None,
        }
    }

    /// List domains with domain server without issue token
    pub async fn list_domains(
        &self,
        org: &str,
    ) -> Result<Vec<DomainWithServer>, Box<dyn std::error::Error + Send + Sync>> {
        let access_token = self
            .api_client
            .get_dds_access_token(self.zitadel_token.as_deref())
            .await?;
        let response = self
            .client
            .get(&format!(
                "{}/api/v1/domains?org={}&with=domain_server",
                self.dds_url, org
            ))
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .header("posemesh-client-id", self.api_client.client_id.clone())
            .send()
            .await?;

        if response.status().is_success() {
            let domain_servers: ListDomainsResponse = response.json().await?;
            Ok(domain_servers.domains)
        } else {
            Err(format!("Failed to list domains. Status: {}", response.status()).into())
        }
    }

    pub async fn sign_in_with_auki_account(
        &mut self,
        email: &str,
        password: &str,
        remember_password: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache.lock().await.clear();
        self.zitadel_token = None;
        let _ = self.api_client.user_login(email, password).await?;
        if remember_password {
            let mut api_client = self.api_client.clone();
            let email = email.to_string();
            let password = password.to_string();
            spawn(async move {
                loop {
                    let expires_at = api_client
                        .get_expires_at()
                        .await
                        .inspect_err(|e| tracing::error!("Failed to get expires at: {}", e));
                    if let Ok(expires_at) = expires_at {
                        let expiration = {
                            let now = now_unix_secs();
                            let duration = expires_at - now;
                            if duration > 600 {
                                Some(Duration::from_secs(duration))
                            } else {
                                None
                            }
                        };

                        if let Some(expiration) = expiration {
                            tracing::info!("Refreshing token in {} seconds", expiration.as_secs());
                            sleep(expiration).await;
                        }

                        let _ = api_client
                            .user_login(&email, &password)
                            .await
                            .inspect_err(|e| tracing::error!("Failed to login: {}", e));
                    }
                }
            });
        }
        Ok(())
    }

    pub async fn sign_in_as_auki_app(
        &mut self,
        app_key: &str,
        app_secret: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache.lock().await.clear();
        self.zitadel_token = None;
        let _ = self
            .api_client
            .sign_in_with_app_credentials(app_key, app_secret)
            .await?;
        Ok(())
    }

    pub fn with_zitadel_token(&self, zitadel_token: &str) -> Self {
        Self {
            dds_url: self.dds_url.clone(),
            client: self.client.clone(),
            cache: Arc::new(Mutex::new(HashMap::new())),
            api_client: AuthClient::new(&self.api_client.api_url, &self.api_client.client_id),
            zitadel_token: Some(zitadel_token.to_string()),
        }
    }

    pub async fn auth_domain(
        &self,
        domain_id: &str,
    ) -> Result<DomainWithToken, Box<dyn std::error::Error + Send + Sync>> {
        let access_token = self
            .api_client
            .get_dds_access_token(self.zitadel_token.as_deref())
            .await?;
        // Check cache first
        let cache = if let Some(cached_domain) = self.cache.lock().await.get(domain_id) {
            cached_domain.clone()
        } else {
            DomainWithToken {
                domain: DomainWithServer {
                    domain: Domain {
                        id: domain_id.to_string(),
                        name: "".to_string(),
                        organization_id: "".to_string(),
                        domain_server_id: "".to_string(),
                        redirect_url: None,
                    },
                    domain_server: DomainServer {
                        id: "".to_string(),
                        organization_id: "".to_string(),
                        name: "".to_string(),
                        url: "".to_string(),
                    },
                },
                expires_at: 0,
                access_token: "".to_string(),
            }
        };

        let cached = get_cached_or_fresh_token(&cache, || {
            let client = self.client.clone();
            let dds_url = self.dds_url.clone();
            let client_id = self.api_client.client_id.clone();
            async move {
                let response = client
                    .post(&format!("{}/api/v1/domains/{}/auth", dds_url, domain_id))
                    .bearer_auth(access_token)
                    .header("Content-Type", "application/json")
                    .header("posemesh-client-id", client_id)
                    .send()
                    .await?;

                if response.status().is_success() {
                    let mut domain_with_token: DomainWithToken = response.json().await?;
                    domain_with_token.expires_at =
                        parse_jwt(&domain_with_token.get_access_token())?.exp;
                    Ok(domain_with_token)
                } else {
                    let status = response.status();
                    let text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    Err(format!("Failed to auth domain. Status: {} - {}", status, text).into())
                }
            }
        })
        .await?;

        // Cache the result
        let mut cache = self.cache.lock().await;
        cache.insert(domain_id.to_string(), cached.clone());
        Ok(cached)
    }
}

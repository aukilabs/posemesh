use std::{collections::HashMap, sync::Arc, time::Duration};

use futures::lock::Mutex;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[cfg(not(target_family = "wasm"))]
use tokio::spawn;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

use posemesh_utils::now_unix_secs;
#[cfg(target_family = "wasm")]
use posemesh_utils::sleep;
#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;

use crate::{auth::{AuthClient, REFRESH_CACHE_TIME, TokenCache, get_cached_or_fresh_token, parse_jwt}, errors::{AukiErrorResponse, DomainError}};
pub const ALL_DOMAINS_ORG: &str = "all";
pub const OWN_DOMAINS_ORG: &str = "own";

#[derive(Debug, Deserialize, Clone, Serialize)]
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
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DomainWithServer {
    pub id: String,
    pub name: String,
    pub organization_id: String,
    pub domain_server_id: String,
    pub redirect_url: Option<String>,
    pub domain_server: DomainServer,
}

#[derive(Debug, Clone)]
pub struct DiscoveryService {
    dds_url: String,
    client: Client,
    cache: Arc<Mutex<HashMap<String, DomainWithToken>>>,
    api_client: AuthClient,
    oidc_access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListDomainsResponse {
    pub domains: Vec<DomainWithServer>,
}

#[derive(Debug, Serialize)]
pub struct CreateDomainRequest {
    pub name: String,
    pub domain_server_id: String,
    pub redirect_url: Option<String>,
    domain_server_url: String,
}

impl DiscoveryService {
    pub fn new(api_url: &str, dds_url: &str, client_id: &str) -> Self {
        let api_client = AuthClient::new(api_url, client_id);

        Self {
            dds_url: dds_url.to_string(),
            client: Client::new(),
            cache: Arc::new(Mutex::new(HashMap::new())),
            api_client,
            oidc_access_token: None,
        }
    }

    /// List domains with domain server without issue token
    pub async fn list_domains(
        &self,
        org: &str,
    ) -> Result<Vec<DomainWithServer>, DomainError> {
        let access_token = self
            .api_client
            .get_dds_access_token(self.oidc_access_token.as_deref())
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
            .header("posemesh-sdk-version", crate::VERSION)
            .send()
            .await?;

        if response.status().is_success() {
            let domain_servers: ListDomainsResponse = response.json().await?;
            Ok(domain_servers.domains)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(AukiErrorResponse { status, error: format!("Failed to list domains. {}", text) }.into())
        }
    }

    pub async fn sign_in_with_auki_account(
        &mut self,
        email: &str,
        password: &str,
        remember_password: bool,
    ) -> Result<String, DomainError> {
        self.cache.lock().await.clear();
        self.oidc_access_token = None;
        let token = self.api_client.user_login(email, password).await?;
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
                            if duration > REFRESH_CACHE_TIME {
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
                            .inspect_err(|e| tracing::error!("Failed to relogin: {}", e));
                    }
                }
            });
        }
        Ok(token)
    }

    pub async fn sign_in_as_auki_app(
        &mut self,
        app_key: &str,
        app_secret: &str,
    ) -> Result<String, DomainError> {
        self.cache.lock().await.clear();
        self.oidc_access_token = None;
        self
            .api_client
            .sign_in_with_app_credentials(app_key, app_secret)
            .await
    }

    pub fn with_oidc_access_token(&self, oidc_access_token: &str) -> Self {
        if let Some(cached_oidc_access_token) = self.oidc_access_token.as_deref() {
            if cached_oidc_access_token == oidc_access_token {
                return self.clone();
            }
        }
        Self {
            dds_url: self.dds_url.clone(),
            client: self.client.clone(),
            cache: Arc::new(Mutex::new(HashMap::new())),
            api_client: AuthClient::new(&self.api_client.api_url, &self.api_client.client_id),
            oidc_access_token: Some(oidc_access_token.to_string()),
        }
    }

    pub async fn auth_domain(
        &self,
        domain_id: &str,
    ) -> Result<DomainWithToken, DomainError> {
        let access_token = self
            .api_client
            .get_dds_access_token(self.oidc_access_token.as_deref())
            .await?;
        // Check cache first
        let cache = if let Some(cached_domain) = self.cache.lock().await.get(domain_id) {
            cached_domain.clone()
        } else {
            DomainWithToken {
                domain: DomainWithServer {
                    id: domain_id.to_string(),
                    name: "".to_string(),
                    organization_id: "".to_string(),
                    domain_server_id: "".to_string(),
                    redirect_url: None,
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
                    .header("posemesh-sdk-version", crate::VERSION)
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
                    Err(AukiErrorResponse { status, error: format!("Failed to auth domain. {}", text) }.into())
                }
            }
        })
        .await?;

        // Cache the result
        let mut cache = self.cache.lock().await;
        cache.insert(domain_id.to_string(), cached.clone());
        Ok(cached)
    }

    pub async fn create_domain(
        &self,
        name: &str,
        domain_server_id: Option<String>,
        domain_server_url: Option<String>,
        redirect_url: Option<String>,
    ) -> Result<DomainWithToken, DomainError> {
        let domain_server_id = domain_server_id.unwrap_or_default();
        let domain_server_url = domain_server_url.unwrap_or_default();
        if domain_server_id.is_empty() && domain_server_url.is_empty() {
            return Err(DomainError::InvalidRequest("domain_server_id or domain_server_url is required"));
        }
        let access_token: String = self
            .api_client
            .get_dds_access_token(self.oidc_access_token.as_deref())
            .await?;
        let response = self
            .client
            .post(&format!("{}/api/v1/domains?issue_token=true", self.dds_url))
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .header("posemesh-client-id", self.api_client.client_id.clone())
            .header("posemesh-sdk-version", crate::VERSION)
            .json(&CreateDomainRequest { name: name.to_string(), domain_server_id: domain_server_id.to_string(), redirect_url, domain_server_url: domain_server_url.to_string() })
            .send()
            .await?;

        if response.status().is_success() {
            let mut domain_with_token: DomainWithToken = response.json().await?;
            domain_with_token.expires_at =
                parse_jwt(&domain_with_token.get_access_token())?.exp;
            // Cache the result
            let mut cache = self.cache.lock().await;
            cache.insert(domain_with_token.domain.id.clone(), domain_with_token.clone());
            Ok(domain_with_token)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(AukiErrorResponse { status, error: format!("Failed to create domain. {}", text) }.into())
        }
    }

    /// List domains by portal, portal_id or portal_short_id is required
    /// If org is not provided, it will list domains for the current authorized organization
    /// If org is provided, it will list domains for the specified organization
    /// Set org to `all` to list domains for all organizations
    pub async fn list_domains_by_portal(
        &self,
        portal_id: Option<&str>,
        portal_short_id: Option<&str>,
        org: Option<&str>,
    ) -> Result<ListDomainsResponse, DomainError> {
        let access_token: String = self
            .api_client
            .get_dds_access_token(self.oidc_access_token.as_deref())
            .await?;
        if portal_id.is_none() && portal_short_id.is_none() {
            return Err(DomainError::InvalidRequest("portal_id or portal_short_id is required"));
        }
        let id = portal_id.or(portal_short_id).unwrap();
        let org = org.unwrap_or(OWN_DOMAINS_ORG);
        let response = self
            .client
            .get(&format!("{}/api/v1/lighthouses/{}/domains?with=domain_server,lighthouse&org={}", self.dds_url, id, org))
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .header("posemesh-client-id", self.api_client.client_id.clone())
            .header("posemesh-sdk-version", crate::VERSION)
            .send()
            .await?;
        if response.status().is_success() {
            let domains: ListDomainsResponse = response.json().await?;
            Ok(domains)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(AukiErrorResponse { status, error: format!("Failed to list domains by portal. {}", text) }.into())
        }
    }

    pub(crate) async fn delete_domain(
        &self,
        access_token: &str,
        domain_id: &str,
    ) -> Result<(), DomainError> {
        let response = self
            .client
            .delete(&format!("{}/api/v1/domains/{}", self.dds_url, domain_id))
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .header("posemesh-client-id", self.api_client.client_id.clone())
            .header("posemesh-sdk-version", crate::VERSION)
            .send()
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(AukiErrorResponse { status, error: format!("Failed to delete domain. {}", text) }.into())
        }
    }
}

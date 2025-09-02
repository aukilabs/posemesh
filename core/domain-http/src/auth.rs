use futures::lock::Mutex;
use reqwest::Client;
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};

use posemesh_utils::now_unix_secs;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AuthClient {
    api_url: String,
    client: Client,
    dds_token_cache: Arc<Mutex<Option<DdsTokenCache>>>,
    user_token_cache: Arc<Mutex<Option<UserTokenCache>>>,
    client_id: String,
    app_key: Option<String>,
    app_secret: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserTokenCache {
    refresh_token: String,
    access_token: String,
    expires_at: u64,
}

impl TokenCache for UserTokenCache {
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

#[derive(Debug, Clone)]
pub(crate) struct DdsTokenCache {
    // DDS access token
    access_token: String,
    // DDS access token expiration time as UTC timestamp
    expires_at: u64,
}

impl TokenCache for DdsTokenCache {
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

pub(crate) trait TokenCache {
    fn get_access_token(&self) -> String;
    fn get_expires_at(&self) -> u64;
    fn set_expires_at(&mut self, expires_at: u64);
}

#[derive(Debug, Serialize)]
pub struct UserCredentials {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UserTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct DdsTokenResponse {
    pub access_token: String,
}

impl AuthClient {
    pub fn new(api_url: &str, client_id: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
            client: Client::new(),
            dds_token_cache: Arc::new(Mutex::new(None)),
            user_token_cache: Arc::new(Mutex::new(None)),
            client_id: client_id.to_string(),
            app_key: None,
            app_secret: None,
        }
    }

    /// Get the expiration time of the user refresh token or DDS access token
    pub async fn get_expires_at(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let token_cache = {
            let cache = self.user_token_cache.lock().await;
            cache.clone()
        };
        if token_cache.is_none() {
            let dds_token_cache = {
                let cache = self.dds_token_cache.lock().await;
                cache.clone()
            };
            if dds_token_cache.is_none() {
                return Err("No token found".into());
            }
            return Ok(dds_token_cache.unwrap().expires_at);
        }
        Ok(parse_jwt(&token_cache.unwrap().refresh_token)?.exp)
    }

    pub async fn set_app_credentials(&mut self, app_key: &str, app_secret: &str) {
        self.app_key = Some(app_key.to_string());
        self.app_secret = Some(app_secret.to_string());
        *self.dds_token_cache.lock().await = None;
        *self.user_token_cache.lock().await = None;
    }

    pub async fn get_dds_access_token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if self.app_key.is_some() {
            return self.get_dds_app_access_token().await;
        } else {
            return self.get_dds_user_access_token().await;
        }
    }

    async fn get_dds_app_access_token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let token_cache = {
            let cache = self.dds_token_cache.lock().await;
            cache.clone()
        };

        let app_key = self.app_key.clone().ok_or("App key is not set".to_string())?;
        let app_secret = self.app_secret.clone().ok_or("App secret is not set".to_string())?;

        let token_cache = get_cached_or_fresh_token(&token_cache.unwrap_or(DdsTokenCache {
            access_token: "".to_string(),
            expires_at: 0,
        }), || {
            let app_key = app_key.to_string();
            let app_secret = app_secret.to_string();
            let client = self.client.clone();
            let api_url = self.api_url.clone();
            let client_id = self.client_id.clone();
            async move {
                let response = client
                    .post(&format!("{}/service/domains-access-token", api_url))
                    .basic_auth(app_key, Some(app_secret))
                    .header("Content-Type", "application/json")
                    .header("posemesh-client-id", client_id)
                    .send()
                    .await?;

                if response.status().is_success() {
                    let token_response: DdsTokenResponse = response.json().await?;
                    Ok(DdsTokenCache {
                        access_token: token_response.access_token.clone(),
                        expires_at: parse_jwt(&token_response.access_token)?.exp,
                    })
                } else {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    Err(format!("Failed to get access token. Status: {} - {}", status, text).into())
                }
            }
        }).await?;

        {
            let mut cache = self.dds_token_cache.lock().await;
            *cache = Some(token_cache.clone());
        }

        Ok(token_cache.access_token)
    }

    async fn get_dds_user_access_token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let token_cache = {
            let cache = self.dds_token_cache.lock().await;
            cache.clone()
        };

        if token_cache.is_none() {
            return Err("No access token found".into());
        }

        let user_token_cache = {
            let cache = self.user_token_cache.lock().await;
            cache.clone()
        };

        if user_token_cache.is_none() {
            return Err("Login first".into());
        }

        let token_cache = get_cached_or_fresh_token(&token_cache.unwrap(), || {
            let client = self.client.clone();
            let api_url = self.api_url.clone();
            let client_id = self.client_id.clone();

            async move {
                let client_clone = client.clone();
                let api_url_clone = api_url.clone();
                let client_id_clone = client_id.clone();
                let refresh_token = user_token_cache.clone().unwrap().refresh_token;
                let user_token_cache = get_cached_or_fresh_token(&user_token_cache.unwrap(), || {
                    async move {
                        let response = client_clone
                            .post(&format!("{}/user/refresh", api_url_clone))
                            .header("Content-Type", "application/json")
                            .header("posemesh-client-id", client_id_clone)
                            .header("Authorization", format!("Bearer {}", refresh_token))
                            .send()
                            .await.expect("Failed to refresh token");
                            
                        if response.status().is_success() {
                            let token_response: UserTokenResponse = response.json().await?; 
                            Ok(UserTokenCache {
                                refresh_token: token_response.refresh_token.clone(),
                                access_token: token_response.access_token.clone(),
                                expires_at: parse_jwt(&token_response.access_token)?.exp,
                            })
                        } else {
                            Err(format!("Failed to refresh token. Status: {}", response.status()).into())
                        }
                    }
                }).await?;

                {
                    let mut cache = self.user_token_cache.lock().await;
                    *cache = Some(user_token_cache.clone());
                }

                let dds_response = client
                    .post(&format!("{}/service/domains-access-token", api_url))
                    .header("Authorization", format!("Bearer {}", user_token_cache.access_token))
                    .header("Content-Type", "application/json")
                    .header("posemesh-client-id", client_id)
                    .send()
                    .await?;

                if dds_response.status().is_success() {
                    let dds_token_response: DdsTokenResponse = dds_response.json().await?;
                    Ok(DdsTokenCache {
                        access_token: dds_token_response.access_token.clone(),
                        expires_at: parse_jwt(&dds_token_response.access_token)?.exp,
                    })
                } else {
                    let status = dds_response.status();
                    let text = dds_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    Err(format!("Failed to get DDS access token. Status: {} - {}", status, text).into())
                }
            }
        }).await?;

        {
            let mut cache = self.dds_token_cache.lock().await;
            *cache = Some(token_cache.clone());
        }

        Ok(token_cache.access_token)
    }

    pub async fn user_login(&mut self, email: &str, password: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let email = email.to_string();
        let password = password.to_string();
        let client = self.client.clone();
        let api_url = self.api_url.clone();
        let client_id = self.client_id.clone();
        let client_id_2 = client_id.clone();
        *self.dds_token_cache.lock().await = None;
        *self.user_token_cache.lock().await = None;
        self.app_key = None;
        self.app_secret = None;
        
        let credentials = UserCredentials {
            email,
            password,
        };

        let response = client
            .post(&format!("{}/user/login", api_url))
            .header("Content-Type", "application/json")
            .header("posemesh-client-id", client_id)
            .json(&credentials)
            .send()
            .await?;

        if response.status().is_success() {
            let token_response: UserTokenResponse = response.json().await?;
            {
                let mut cache = self.user_token_cache.lock().await;
                *cache = Some(UserTokenCache {
                    refresh_token: token_response.refresh_token.clone(),
                    access_token: token_response.access_token.clone(),
                    expires_at: parse_jwt(&token_response.access_token)?.exp,
                });
            }

            let dds_response = client
                .post(&format!("{}/service/domains-access-token", api_url))
                .header("Authorization", format!("Bearer {}", token_response.access_token))
                .header("Content-Type", "application/json")
                .header("posemesh-client-id", client_id_2)
                .send()
                .await?;

            if dds_response.status().is_success() {
                let dds_token_response: DdsTokenResponse = dds_response.json().await?;
                let mut cache = self.dds_token_cache.lock().await;
                *cache = Some(DdsTokenCache {
                    access_token: dds_token_response.access_token.clone(),
                    expires_at: parse_jwt(&dds_token_response.access_token)?.exp,
                });
                Ok(dds_token_response.access_token)
            } else {
                let status = dds_response.status();
                let text = dds_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("Failed to get DDS access token. Status: {} - {}", status, text).into())
            }
        } else {
            Err(format!("Failed to login. Status: {}", response.status()).into())
        }
    }

}

const REFRESH_CACHE_TIME: u64 = 3;

pub(crate) async fn get_cached_or_fresh_token<R, F, Fut>(cache: &R, token_fetcher: F) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce() -> Fut,
    R: TokenCache + Clone,
    Fut: std::future::Future<Output = Result<R, Box<dyn std::error::Error + Send + Sync>>>,
{
    // Check if we have a valid cached token
    let expires_at = cache.get_expires_at();
    let current_time = now_unix_secs();
    // If token expires in more than REFRESH_CACHE_TIME seconds, return cached token
    if expires_at > current_time && expires_at - current_time > REFRESH_CACHE_TIME {
        return Ok(cache.clone());
    }

    // Fetch new token
    let mut token_response = token_fetcher().await?;

    let claim = parse_jwt(&token_response.get_access_token())?;
    // Use the actual expiration from JWT if available
    token_response.set_expires_at(claim.exp);

    Ok(token_response)
}

#[derive(Debug, Deserialize)]
pub struct JwtClaim {
    pub exp: u64,
}

pub fn parse_jwt(token: &str) -> Result<JwtClaim, Box<dyn std::error::Error + Send + Sync>> {
    let parts = token.split('.').collect::<Vec<&str>>();
    if parts.len() != 3 {
        return Err("Invalid JWT token".into());
    }
    let payload = parts[1];
    let decoded = general_purpose::URL_SAFE_NO_PAD.decode(payload)?;
    let claims: JwtClaim = serde_json::from_slice(&decoded)?;
    Ok(claims)
}

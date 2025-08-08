use futures::lock::Mutex;
use reqwest::Client;
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};

use std::{sync::Arc, time};

#[derive(Debug, Clone)]
pub struct AuthClient {
    api_url: String,
    client: Client,
    token_cache: Arc<Mutex<DdsTokenCache>>,
    client_id: String,
    app_key: Option<String>,
    app_secret: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct DdsTokenCache {
    // User refresh token
    refresh_token: Option<String>,
    // DDS access token
    access_token: Option<String>,
    // DDS access token expiration time as UTC timestamp
    expires_at: Option<u64>,
}

impl TokenCache for DdsTokenCache {
    fn get_access_token(&self) -> Option<String> {
        self.access_token.clone()
    }

    fn get_expires_at(&self) -> Option<u64> {
        self.expires_at
    }

    fn set_expires_at(&mut self, expires_at: u64) {
        self.expires_at = Some(expires_at);
    }
}

pub(crate) trait TokenCache {
    fn get_access_token(&self) -> Option<String>;
    fn get_expires_at(&self) -> Option<u64>;
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
            token_cache: Arc::new(Mutex::new(DdsTokenCache {
                access_token: None,
                expires_at: None,
                refresh_token: None,
            })),
            client_id: client_id.to_string(),
            app_key: None,
            app_secret: None,
        }
    }

    pub async fn set_app_credentials(&mut self, app_key: &str, app_secret: &str) {
        self.app_key = Some(app_key.to_string());
        self.app_secret = Some(app_secret.to_string());
        self.token_cache.lock().await.access_token = None;
        self.token_cache.lock().await.expires_at = None;
        self.token_cache.lock().await.refresh_token = None;
    }

    pub async fn get_dds_access_token(&self) -> Result<String, Box<dyn std::error::Error>> {
        if self.app_key.is_some() {
            return self.get_dds_app_access_token().await;
        } else {
            return self.get_dds_user_access_token().await;
        }
    }

    async fn get_dds_app_access_token(&self) -> Result<String, Box<dyn std::error::Error>> {
        let token_cache = {
            let cache = self.token_cache.lock().await;
            DdsTokenCache {
                refresh_token: cache.refresh_token.clone(),
                access_token: cache.access_token.clone(),
                expires_at: cache.expires_at.clone(),
            }
        };

        let app_key = self.app_key.clone().ok_or("App key is not set".to_string())?;
        let app_secret = self.app_secret.clone().ok_or("App secret is not set".to_string())?;

        let token_cache = get_cached_or_fresh_token(&token_cache, || {
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
                        refresh_token: None,
                        access_token: Some(token_response.access_token.clone()),
                        expires_at: None,
                    })
                } else {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    Err(format!("Failed to get access token. Status: {} - {}", status, text).into())
                }
            }
        }).await?;

        {
            let mut cache = self.token_cache.lock().await;
            cache.access_token = token_cache.access_token.clone();
            cache.expires_at = token_cache.expires_at.clone();
        }

        Ok(token_cache.access_token.unwrap())
    }

    async fn get_dds_user_access_token(&self) -> Result<String, Box<dyn std::error::Error>> {
        let token_cache = {
            let cache = self.token_cache.lock().await;
            DdsTokenCache {
                refresh_token: cache.refresh_token.clone(),
                access_token: cache.access_token.clone(),
                expires_at: cache.expires_at.clone(),
            }
        };

        if token_cache.access_token.is_none() {
            return Err("No access token found".into());
        }

        if token_cache.refresh_token.is_none() {
            return Err("No refresh token found".into());
        }

        let refresh_token = token_cache.refresh_token.clone().unwrap();

        let token_cache = get_cached_or_fresh_token(&token_cache, || {
            let client = self.client.clone();
            let api_url = self.api_url.clone();
            let client_id = self.client_id.clone();
            async move {
                let response = client
                    .post(&format!("{}/user/refresh", api_url))
                    .header("Content-Type", "application/json")
                    .header("posemesh-client-id", client_id)
                    .header("Authorization", format!("Bearer {}", refresh_token))
                    .send()
                    .await.expect("Failed to refresh token");

                if response.status().is_success() {
                    let token_response: UserTokenResponse = response.json().await?;
                    Ok(DdsTokenCache {
                        refresh_token: None,
                        access_token: Some(token_response.access_token.clone()),
                        expires_at: None,
                    })
                } else {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    Err(format!("Failed to refresh token. Status: {} - {}", status, text).into())
                }
            }
        }).await?;

        {
            let mut cache = self.token_cache.lock().await;
            cache.access_token = token_cache.access_token.clone();
            cache.expires_at = token_cache.expires_at.clone();
        }

        Ok(token_cache.access_token.unwrap())
    }

    pub async fn user_login(&mut self, email: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {
        let email = email.to_string();
        let password = password.to_string();
        let client = self.client.clone();
        let api_url = self.api_url.clone();
        let client_id = self.client_id.clone();
        let client_id_2 = client_id.clone();
        self.token_cache.lock().await.access_token = None;
        self.token_cache.lock().await.expires_at = None;
        self.token_cache.lock().await.refresh_token = None;
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
                let mut cache = self.token_cache.lock().await;
                cache.refresh_token = Some(token_response.refresh_token.clone());
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
                let mut cache = self.token_cache.lock().await;
                cache.access_token = Some(dds_token_response.access_token.clone());
                cache.expires_at = Some(parse_jwt(&dds_token_response.access_token)?.exp);
                cache.refresh_token = Some(token_response.refresh_token.clone());
                Ok(dds_token_response.access_token)
            } else {
                Err(format!("Failed to get DDS access token. Status: {}", dds_response.status()).into())
            }
        } else {
            Err(format!("Failed to login. Status: {}", response.status()).into())
        }
    }

}


pub(crate) async fn get_cached_or_fresh_token<R, F, Fut>(cache: &R, token_fetcher: F) -> Result<R, Box<dyn std::error::Error>>
where
    F: FnOnce() -> Fut,
    R: TokenCache + Clone, 
    Fut: std::future::Future<Output = Result<R, Box<dyn std::error::Error>>>,
{
    // Check if we have a valid cached token
    if let Some(expires_at) = cache.get_expires_at() {
        // If token expires in more than 5 minutes, return cached token
        if expires_at - time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_secs() > 300 {
            return Ok(cache.clone());
        }
    }

    // Fetch new token
    let mut token_response = token_fetcher().await?;

    let claim = parse_jwt(&token_response.get_access_token().unwrap())?;
    // Use the actual expiration from JWT if available
    token_response.set_expires_at(claim.exp);

    Ok(token_response)
}

#[derive(Debug, Deserialize)]
pub struct JwtClaim {
    exp: u64,
}

pub fn parse_jwt(token: &str) -> Result<JwtClaim, Box<dyn std::error::Error>> {
    let parts = token.split('.').collect::<Vec<&str>>();
    if parts.len() != 3 {
        return Err("Invalid JWT token".into());
    }
    let payload = parts[1];
    let decoded = general_purpose::URL_SAFE_NO_PAD.decode(payload)?;
    let claims: JwtClaim = serde_json::from_slice(&decoded)?;
    Ok(claims)
}

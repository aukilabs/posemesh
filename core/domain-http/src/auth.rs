use base64::{Engine as _, engine::general_purpose};
use futures::lock::Mutex;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use posemesh_utils::now_unix_secs;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AuthClient {
    pub api_url: String,
    client: Client,
    dds_token_cache: Arc<Mutex<Option<DdsTokenCache>>>,
    user_token_cache: Arc<Mutex<Option<UserTokenCache>>>,
    pub client_id: String,
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
}

pub(crate) trait TokenCache {
    fn get_access_token(&self) -> String;
    fn get_expires_at(&self) -> u64;
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

    pub async fn sign_in_with_app_credentials(
        &mut self,
        app_key: &str,
        app_secret: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.app_key = Some(app_key.to_string());
        self.app_secret = Some(app_secret.to_string());
        *self.dds_token_cache.lock().await = None;
        *self.user_token_cache.lock().await = None;

        self.get_dds_app_access_token().await
    }

    // Get DDS access token with either app credentials or user access token or oidc_access_token, it checks the cache first, if found and not about to expire, return the cached token
    // if not found or about to expire, it fetches a new token with app credentials or user access token or oidc_access_token and sets the cache.
    // If user access token is about to expire, it refreshes the user access token with refresh token first and sets the cache.
    // It clears all caches if there is an error.
    pub async fn get_dds_access_token(
        &self,
        oidc_access_token: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let result = if let Some(oidc_access_token) = oidc_access_token {
            self.get_dds_access_token_with_oidc_access_token(oidc_access_token).await
        } else if self.app_key.is_some() {
            self.get_dds_app_access_token().await
        } else {
            self.get_dds_user_access_token().await
        };

        if result.is_err() {
            *self.dds_token_cache.lock().await = None;
            *self.user_token_cache.lock().await = None;
        }

        result
    }

    // Get DDS access token with OIDC access token, doesn't cache
    async fn get_dds_access_token_with_oidc_access_token(
        &self,
        oidc_access_token: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Clear all caches before proceeding
        *self.dds_token_cache.lock().await = None;
        *self.user_token_cache.lock().await = None;
        
        let response = self.get_dds_token_by_token(oidc_access_token).await?;
        {
            let mut cache = self.dds_token_cache.lock().await;
            *cache = Some(DdsTokenCache {
                access_token: response.access_token.clone(),
                expires_at: parse_jwt(&response.access_token)?.exp,
            });
        }
        Ok(response.access_token)
    }

    // Get DDS access token with app credentials, it checks the cache first, if found and not about to expire, return the cached token
    // if not found or about to expire, fetch a new token with app credentials and sets the cache.
    async fn get_dds_app_access_token(
        &self,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let token_cache = {
            let cache = self.dds_token_cache.lock().await;
            cache.clone()
        };

        let app_key = self
            .app_key
            .clone()
            .ok_or("App key is not set".to_string())?;
        let app_secret = self
            .app_secret
            .clone()
            .ok_or("App secret is not set".to_string())?;

        let token_cache = get_cached_or_fresh_token(
            &token_cache.unwrap_or(DdsTokenCache {
                access_token: "".to_string(),
                expires_at: 0,
            }),
            || {
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
                        let text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        Err(format!(
                            "Failed to get DDS access token. Status: {} - {}",
                            status, text
                        )
                        .into())
                    }
                }
            },
        )
        .await?;

        {
            let mut cache = self.dds_token_cache.lock().await;
            *cache = Some(token_cache.clone());
        }

        Ok(token_cache.access_token)
    }

    // Get DDS access token with user credentials, it checks the cache first, if found and not about to expire, return the cached token
    // if not found or about to expire, it fetches a new token with user access token and sets the cache.
    // If user access token is about to expire, it refreshes the user access token with refresh token first and sets the cache.
    async fn get_dds_user_access_token(
        &self,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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
                let user_token_cache =
                    get_cached_or_fresh_token(&user_token_cache.unwrap(), || async move {
                        let response = client_clone
                            .post(&format!("{}/user/refresh", api_url_clone))
                            .header("Content-Type", "application/json")
                            .header("posemesh-client-id", client_id_clone)
                            .header("Authorization", format!("Bearer {}", refresh_token))
                            .send()
                            .await
                            .expect("Failed to refresh token");

                        if response.status().is_success() {
                            let token_response: UserTokenResponse = response.json().await?;
                            Ok(UserTokenCache {
                                refresh_token: token_response.refresh_token.clone(),
                                access_token: token_response.access_token.clone(),
                                expires_at: parse_jwt(&token_response.access_token)?.exp,
                            })
                        } else {
                            Err(
                                format!("Failed to refresh token. Status: {}", response.status())
                                    .into(),
                            )
                        }
                    })
                    .await?;

                {
                    let mut cache = self.user_token_cache.lock().await;
                    *cache = Some(user_token_cache.clone());
                }

                let dds_token_response = self.get_dds_token_by_token(&user_token_cache.access_token).await?;

                let dds_cache = DdsTokenCache {
                    access_token: dds_token_response.access_token.clone(),
                    expires_at: parse_jwt(&dds_token_response.access_token)?.exp,
                };
                {
                    let mut cache = self.dds_token_cache.lock().await;
                    *cache = Some(dds_cache.clone());
                }
                Ok(dds_cache)
            }
        })
        .await?;
    
        {
            let mut cache = self.dds_token_cache.lock().await;
            *cache = Some(token_cache.clone());
        }

        Ok(token_cache.access_token)
    }

    // Login with user credentials, return DDS access token. It clears all caches and sets the app credentials to none.
    pub async fn user_login(
        &mut self,
        email: &str,
        password: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        *self.dds_token_cache.lock().await = None;
        *self.user_token_cache.lock().await = None;
        self.app_key = None;
        self.app_secret = None;

        let credentials = UserCredentials { email: email.to_string(), password: password.to_string() };

        let response = self.client
            .post(&format!("{}/user/login", &self.api_url))
            .header("Content-Type", "application/json")
            .header("posemesh-client-id", &self.client_id)
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

            let dds_token_response = self.get_dds_token_by_token(&token_response.access_token).await?;
            let mut cache = self.dds_token_cache.lock().await;
            let token_cache = DdsTokenCache {
                access_token: dds_token_response.access_token.clone(),
                expires_at: parse_jwt(&dds_token_response.access_token)?.exp,
            };
            *cache = Some(token_cache.clone());
            Ok(token_cache.access_token)
        } else {
            Err(format!("Failed to login. Status: {}", response.status()).into())
        }
    }

    // Get DDS access token with either user access token or oidc_access_token, doesn't cache
    async fn get_dds_token_by_token(
        &self,
        token: &str,
    ) -> Result<DdsTokenResponse, Box<dyn std::error::Error + Send + Sync>> {
        let dds_response = self.client.post(&format!("{}/service/domains-access-token", &self.api_url))
            .header(
                "Authorization",
                format!("Bearer {}", token),
            )
            .header("Content-Type", "application/json")
            .header("posemesh-client-id", &self.client_id)
            .send()
            .await?;

        if dds_response.status().is_success() {
            dds_response.json::<DdsTokenResponse>().await.map_err(|e| e.into())
        } else {
            let status = dds_response.status();
            let text = dds_response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!(
                "Failed to get DDS access token. Status: {} - {}",
                status, text
            )
            .into())
        }
    }
}

const REFRESH_CACHE_TIME: u64 = 3;

pub(crate) async fn get_cached_or_fresh_token<R, F, Fut>(
    cache: &R,
    token_fetcher: F,
) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
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
    token_fetcher().await
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Clone, Debug)]
    struct DummyTokenCache {
        access_token: String,
        expires_at: u64,
    }

    impl TokenCache for DummyTokenCache {
        fn get_access_token(&self) -> String {
            self.access_token.clone()
        }
        fn get_expires_at(&self) -> u64 {
            self.expires_at
        }
    }

    fn now_unix_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn make_jwt(exp: u64) -> String {
        // Header: {"alg":"HS256","typ":"JWT"}
        // Payload: {"exp":exp}
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{}}}"#, exp));
        format!("{}.{}.sig", header, payload)
    }

    #[tokio::test]
    async fn test_ddstoken_about_to_expire_should_refetch() {
        // Token expires in 2 seconds (less than REFRESH_CACHE_TIME)
        let now = now_unix_secs();
        let expiring_soon = now + 2;
        let cache = DummyTokenCache {
            access_token: make_jwt(expiring_soon),
            expires_at: expiring_soon,
        };

        let fetch_called = Arc::new(Mutex::new(false));
        let fetch_called_clone = fetch_called.clone();

        let new_exp = now + 1000;
        let token_fetcher = move || {
            let fetch_called_clone = fetch_called_clone.clone();
            async move {
                *fetch_called_clone.lock().await = true;
                let token = DummyTokenCache {
                    access_token: make_jwt(new_exp),
                    expires_at: new_exp,
                };
                // set_expires_at will be called by get_cached_or_fresh_token
                Ok(token)
            }
        };

        let result = get_cached_or_fresh_token(&cache, token_fetcher).await.unwrap();
        // Should have called fetcher
        assert!(*fetch_called.lock().await, "Fetcher should have been called");
        // Should have new expiration
        assert_eq!(result.expires_at, new_exp);
    }

    #[tokio::test]
    async fn test_ddstoken_not_expiring_should_use_cache() {
        // Token expires in 100 seconds (more than REFRESH_CACHE_TIME)
        let now = now_unix_secs();
        let not_expiring = now + 100;
        let cache = DummyTokenCache {
            access_token: make_jwt(not_expiring),
            expires_at: not_expiring,
        };

        let fetch_called = Arc::new(Mutex::new(false));
        let fetch_called_clone = fetch_called.clone();

        let cache_clone = cache.clone();
        let token_fetcher = move || {
            let fetch_called_clone = fetch_called_clone.clone();
            async move {
                *fetch_called_clone.lock().await = true;
                Ok(cache_clone.clone())
            }
        };

        let result = get_cached_or_fresh_token(&cache, token_fetcher).await.unwrap();
        // Should NOT have called fetcher
        assert!(!*fetch_called.lock().await, "Fetcher should NOT have been called");
        // Should have same expiration
        assert_eq!(result.expires_at, not_expiring);
    }
}

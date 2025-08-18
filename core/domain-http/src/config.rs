use serde::Deserialize;
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_family = "wasm", wasm_bindgen(getter_with_clone))]
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub api_url: String,
    pub dds_url: String,
    pub client_id: String,
    pub app_key: Option<String>,
    pub app_secret: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, std::env::VarError> {
        Ok(Config {
            api_url: std::env::var("API_URL")?,
            dds_url: std::env::var("DDS_URL")?,
            client_id: std::env::var("CLIENT_ID")?,
            app_key: std::env::var("APP_KEY").map_or(None, |v| Some(v)),
            app_secret: std::env::var("APP_SECRET").map_or(None, |v| Some(v)),
            email: std::env::var("POSEMESH_EMAIL").map_or(None, |v| Some(v)),
            password: std::env::var("POSEMESH_PASSWORD").map_or(None, |v| Some(v)),
        })
    }
}

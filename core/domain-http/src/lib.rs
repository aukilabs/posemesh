use futures::channel::mpsc::Receiver;

#[cfg(target_family = "wasm")]
use crate::domain_data::{UploadDomainData, upload_v1};
use crate::domain_data::{download_metadata_v1, download_v1_stream, DomainData, DownloadQuery};

mod auth;
pub mod config;
pub mod domain_data;
pub mod discovery;
#[cfg(target_family = "wasm")]
pub mod wasm;

use crate::discovery::DiscoveryService;

#[derive(Debug, Clone)]
pub struct DomainClient {
    discovery_client: DiscoveryService,
    pub client_id: String,
}

impl DomainClient {
    fn new(api_url: &str, dds_url: &str, client_id: &str) -> Self {
        if client_id.is_empty() {
            panic!("client_id is empty");
        }
        Self {
            discovery_client: DiscoveryService::new(api_url, dds_url, client_id),
            client_id: client_id.to_string(),
        }
    }

    pub async fn new_with_app_credential(api_url: &str, dds_url: &str, client_id: &str, app_key: &str, app_secret: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut dc = DomainClient::new(api_url, dds_url, client_id);
        let _ = dc.discovery_client.sign_in_as_auki_app(app_key, app_secret).await?;
        Ok(dc)
    }

    pub async fn new_with_user_credential(api_url: &str, dds_url: &str, client_id: &str, email: &str, password: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut dc = DomainClient::new(api_url, dds_url, client_id);
        let _ = dc.discovery_client.sign_in_with_auki_account(email, password).await?;
        Ok(dc)
    }

    pub async fn download_domain_data(&self, domain_id: &str, query: &DownloadQuery) -> Result<Receiver<Result<DomainData, Box<dyn std::error::Error + Send + Sync>>>, Box<dyn std::error::Error + Send + Sync>> {
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        let rx = download_v1_stream(&domain.domain.domain_server.url, &self.client_id, &domain.access_token, domain_id, query).await?;
        Ok(rx)
    }

    #[cfg(not(target_family = "wasm"))]
    pub async fn upload_domain_data(&self, domain_id: &str, data: Receiver<domain_data::UploadDomainData>) -> Result<Vec<DomainData>, Box<dyn std::error::Error + Send + Sync>> {
        use crate::domain_data::upload_v1_stream;
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        upload_v1_stream(&domain.domain.domain_server.url, &domain.access_token, domain_id, data).await
    }

    #[cfg(target_family = "wasm")]
    pub async fn upload_domain_data(&self, domain_id: &str, data: Vec<UploadDomainData>) -> Result<Vec<DomainData>, Box<dyn std::error::Error + Send + Sync>> {
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        upload_v1(&domain.domain.domain_server.url, &domain.access_token, domain_id, data).await
    }

    pub async fn download_metadata(&self, domain_id: &str, query: &DownloadQuery) -> Result<Vec<DomainData>, Box<dyn std::error::Error + Send + Sync>> {
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        download_metadata_v1(&domain.domain.domain_server.url, &self.client_id, &domain.access_token, domain_id, query).await
    }
}

#[cfg(not(target_family = "wasm"))]
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::domain_data::{UpdateDomainData, UploadDomainData};

    use super::*;
    use futures::{channel::mpsc, StreamExt};
    use tokio::{spawn, time::sleep};

    fn get_config() -> (config::Config, String) {
        if std::path::Path::new("../.env.local").exists() {
            dotenvy::from_filename("../.env.local").ok();
            dotenvy::dotenv().ok();
        }
        let config = config::Config::from_env().unwrap();
        (config, std::env::var("DOMAIN_ID").unwrap())
    }

    #[tokio::test]
    async fn test_download_domain_data_with_app_credential() {
        // Create a test client
        let config = get_config();
        let client = DomainClient::new_with_app_credential(&config.0.api_url, &config.0.dds_url, &config.0.client_id, &config.0.app_key.unwrap(), &config.0.app_secret.unwrap()).await.expect("Failed to create client");
        
        // Create a test query
        let query = DownloadQuery {
            ids: vec![],
            name: None,
            data_type: Some("dmt_accel_csv".to_string()),
        };

        // Test the download function
        let result = client.download_domain_data(
            &config.1,
            &query
        ).await;

        assert!(result.is_ok(), "error message : {:?}", result.err());

        let mut count = 0;
        let mut rx = result.unwrap();
        while let Some(Ok(_)) = rx.next().await {
            count += 1;
        }
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_upload_domain_data_with_user_credential() {
        use futures::SinkExt;
        let config = get_config();
        let client = DomainClient::new_with_user_credential(&config.0.api_url, &config.0.dds_url, &config.0.client_id, &config.0.email.unwrap(), &config.0.password.unwrap()).await.expect("Failed to create client");

        let data = vec![UploadDomainData {
            create: None,
            update: Some(UpdateDomainData {
                id: "a84a36e5-312b-4f80-974a-06f5d19c1e16".to_string(),
            }),
            data: "{\"test\": \"test updated\"}".as_bytes().to_vec(),
        }, UploadDomainData {
            create: None,
            update: Some(UpdateDomainData {
                id: "a08dc12f-c79e-4f5e-b388-c09ad6d8cfd8".to_string(),
            }),
            data: "{\"test\": \"test updated\"}".as_bytes().to_vec(),
        }];
        let (mut tx, rx) = mpsc::channel(10);
        spawn(async move {
            for d in data {
                tx.send(d).await.unwrap();
            }
            tx.close().await.unwrap();
        });
        let result = client.upload_domain_data(&config.1, rx).await;

        assert!(result.is_ok(), "error message : {:?}", result.err());
        assert_eq!(result.unwrap().len(), 2);

        sleep(Duration::from_secs(5)).await;

        // Create a test query
        let query = DownloadQuery {
            ids: vec![],
            name: None,
            data_type: Some("dmt_accel_csv".to_string()),
        };

        // Test the download function
        let result = client.download_domain_data(
            &config.1,
            &query
        ).await;

        assert!(result.is_ok(), "error message : {:?}", result.err());

        let mut count = 0;
        let mut rx = result.unwrap();
        while let Some(Ok(_)) = rx.next().await {
            count += 1;
        }
        assert!(count > 0);
    }
}


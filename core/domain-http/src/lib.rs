use futures::channel::mpsc::Receiver;
use crate::domain_data::{
    DomainData, DomainDataMetadata, DownloadQuery, delete_by_id, download_by_id,
    download_metadata_v1, download_v1_stream,
};

#[cfg(target_family = "wasm")]
use crate::domain_data::{UploadDomainData, upload_v1};

mod auth;
pub mod config;
pub mod discovery;
pub mod domain_data;
pub mod reconstruction;
#[cfg(target_family = "wasm")]
pub mod wasm;
pub mod errors;

use crate::auth::TokenCache;
use crate::discovery::{DiscoveryService, DomainWithServer};
use crate::errors::DomainError;
pub use crate::reconstruction::JobRequest;


#[derive(Debug, Clone)]
pub struct DomainClient {
    discovery_client: DiscoveryService,
    pub client_id: String,
}

impl DomainClient {
    pub fn new(api_url: &str, dds_url: &str, client_id: &str) -> Self {
        if client_id.is_empty() {
            panic!("client_id is empty");
        }
        Self {
            discovery_client: DiscoveryService::new(api_url, dds_url, client_id),
            client_id: client_id.to_string(),
        }
    }

    pub async fn new_with_app_credential(
        api_url: &str,
        dds_url: &str,
        client_id: &str,
        app_key: &str,
        app_secret: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut dc = DomainClient::new(api_url, dds_url, client_id);
        let _ = dc
            .discovery_client
            .sign_in_as_auki_app(app_key, app_secret)
            .await?;
        Ok(dc)
    }

    pub async fn new_with_user_credential(
        api_url: &str,
        dds_url: &str,
        client_id: &str,
        email: &str,
        password: &str,
        remember_password: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut dc = DomainClient::new(api_url, dds_url, client_id);
        let _ = dc
            .discovery_client
            .sign_in_with_auki_account(email, password, remember_password)
            .await?;
        Ok(dc)
    }

    pub fn with_oidc_access_token(&self, token: &str) -> Self {
        Self {
            discovery_client: self.discovery_client.with_oidc_access_token(token),
            client_id: self.client_id.clone(),
        }
    }

    pub async fn download_domain_data(
        &self,
        domain_id: &str,
        query: &DownloadQuery,
    ) -> Result<
        Receiver<Result<DomainData, DomainError>>,
        DomainError,
    > {
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        let rx = download_v1_stream(
            &domain.domain.domain_server.url,
            &self.client_id,
            &domain.get_access_token(),
            domain_id,
            query,
        )
        .await?;
        Ok(rx)
    }

    #[cfg(not(target_family = "wasm"))]
    pub async fn upload_domain_data(
        &self,
        domain_id: &str,
        data: Receiver<domain_data::UploadDomainData>,
    ) -> Result<Vec<DomainDataMetadata>, DomainError> {
        use crate::{auth::TokenCache, domain_data::upload_v1_stream};
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        upload_v1_stream(
            &domain.domain.domain_server.url,
            &domain.get_access_token(),
            domain_id,
            data,
        )
        .await
    }

    #[cfg(target_family = "wasm")]
    pub async fn upload_domain_data(
        &self,
        domain_id: &str,
        data: Vec<UploadDomainData>,
    ) -> Result<Vec<DomainDataMetadata>, DomainError> {
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        upload_v1(
            &domain.domain.domain_server.url,
            &domain.get_access_token(),
            domain_id,
            data,
        )
        .await
    }

    pub async fn download_metadata(
        &self,
        domain_id: &str,
        query: &DownloadQuery,
    ) -> Result<Vec<DomainDataMetadata>, DomainError> {
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        download_metadata_v1(
            &domain.domain.domain_server.url,
            &self.client_id,
            &domain.get_access_token(),
            domain_id,
            query,
        )
        .await
    }

    pub async fn download_domain_data_by_id(
        &self,
        domain_id: &str,
        id: &str,
    ) -> Result<Vec<u8>, DomainError> {
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        download_by_id(
            &domain.domain.domain_server.url,
            &self.client_id,
            &domain.get_access_token(),
            domain_id,
            id,
        )
        .await
    }

    pub async fn delete_domain_data_by_id(
        &self,
        domain_id: &str,
        id: &str,
    ) -> Result<(), DomainError> {
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        delete_by_id(
            &domain.domain.domain_server.url,
            &domain.get_access_token(),
            domain_id,
            id,
        )
        .await
    }

    pub async fn submit_job_request_v1(
        &self,
        domain_id: &str,
        request: &JobRequest,
    ) -> Result<reqwest::Response, DomainError> {
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        crate::reconstruction::forward_job_request_v1(
            &domain.domain.domain_server.url,
            &self.client_id,
            &domain.get_access_token(),
            domain_id,
            request,
        )
        .await
    }
  
    pub async fn list_domains(
        &self,
        org: &str,
    ) -> Result<Vec<DomainWithServer>, DomainError> {
        self.discovery_client.list_domains(org).await
    }
}

#[cfg(not(target_family = "wasm"))]
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::domain_data::{CreateDomainData, DomainAction, UpdateDomainData, UploadDomainData};

    use super::*;
    use futures::{StreamExt, channel::mpsc};
    use tokio::{spawn, time::sleep};

    fn get_config() -> (config::Config, String) {
        if std::path::Path::new("../.env.local").exists() {
            dotenvy::from_filename("../.env.local").ok();
        }
        dotenvy::dotenv().ok();
        let config = config::Config::from_env().unwrap();
        (config, std::env::var("DOMAIN_ID").unwrap())
    }

    #[tokio::test]
    async fn test_download_domain_data_with_app_credential() {
        // Create a test client
        let config = get_config();
        let client = DomainClient::new_with_app_credential(
            &config.0.api_url,
            &config.0.dds_url,
            &config.0.client_id,
            &config.0.app_key.unwrap(),
            &config.0.app_secret.unwrap(),
        )
        .await
        .expect("Failed to create client");

        // Create a test query
        let query = DownloadQuery {
            ids: vec![],
            name: None,
            data_type: Some("dmt_accel_csv".to_string()),
        };

        // Test the download function
        let result = client.download_domain_data(&config.1, &query).await;

        assert!(result.is_ok(), "error message : {:?}", result.err());

        let mut rx = result.unwrap();
        let mut count = 0;
        while let Some(Ok(data)) = rx.next().await {
            count += 1;
            assert_eq!(data.metadata.data_type, "dmt_accel_csv");
        }
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_upload_domain_data_with_user_credential() {
        use futures::SinkExt;
        let config = get_config();
        let client = DomainClient::new_with_user_credential(
            &config.0.api_url,
            &config.0.dds_url,
            &config.0.client_id,
            &config.0.email.unwrap(),
            &config.0.password.unwrap(),
            true,
        )
        .await
        .expect("Failed to create client");

        let data = vec![
            UploadDomainData {
                action: DomainAction::Create(CreateDomainData {
                    name: "to be deleted".to_string(),
                    data_type: "test".to_string(),
                }),
                data: "{\"test\": \"test\"}".as_bytes().to_vec(),
            },
            UploadDomainData {
                action: DomainAction::Update(UpdateDomainData {
                    id: "a84a36e5-312b-4f80-974a-06f5d19c1e16".to_string(),
                }),
                data: "{\"test\": \"test updated\"}".as_bytes().to_vec(),
            },
        ];
        let (mut tx, rx) = mpsc::channel(10);
        spawn(async move {
            for d in data {
                tx.send(d).await.unwrap();
            }
            tx.close().await.unwrap();
        });
        let result = client.upload_domain_data(&config.1, rx).await;

        assert!(result.is_ok(), "error message : {:?}", result.err());

        sleep(Duration::from_secs(5)).await;
        let result = result.unwrap();
        assert_eq!(result.len(), 2);

        let ids = result.iter().map(|d| d.id.clone()).collect::<Vec<String>>();
        assert_eq!(ids.len(), 2);
        // Create a test query
        let query = DownloadQuery {
            ids: ids,
            name: None,
            data_type: None,
        };

        // Test the download function
        let result = client.download_domain_data(&config.1, &query).await;

        assert!(result.is_ok(), "error message : {:?}", result.err());

        let mut to_delete = None;
        let mut count = 0;
        let mut rx = result.unwrap();
        while let Some(Ok(data)) = rx.next().await {
            count += 1;
            if data.metadata.id == "a84a36e5-312b-4f80-974a-06f5d19c1e16" {
                assert_eq!(data.data, b"{\"test\": \"test updated\"}");
                continue;
            } else {
                assert_eq!(data.data, b"{\"test\": \"test\"}");
            }
            to_delete = Some(data.metadata.id.clone());
        }
        assert_eq!(count, 2);
        assert_eq!(to_delete.is_some(), true);

        // Delete the one whose id is not "a8"
        let delete_result = client
            .delete_domain_data_by_id(&config.1, &to_delete.unwrap())
            .await;
        assert!(
            delete_result.is_ok(),
            "Failed to delete data by id: {:?}",
            delete_result.err()
        );
    }

    #[tokio::test]
    async fn test_download_domain_data_by_id() {
        let config = get_config();
        let client = DomainClient::new_with_app_credential(
            &config.0.api_url,
            &config.0.dds_url,
            &config.0.client_id,
            &config.0.app_key.unwrap(),
            &config.0.app_secret.unwrap(),
        )
        .await
        .expect("Failed to create client");

        // Now test download by id
        let download_result = client
            .download_domain_data_by_id(&config.1, "a84a36e5-312b-4f80-974a-06f5d19c1e16")
            .await;

        assert!(
            download_result.is_ok(),
            "download by id failed: {:?}",
            download_result.err()
        );
        let downloaded_bytes = download_result.unwrap();
        assert_eq!(downloaded_bytes, b"{\"test\": \"test updated\"}".to_vec());
    }

    #[tokio::test]
    async fn test_download_domain_metadata() {
        let config = get_config();
        let client = DomainClient::new_with_app_credential(
            &config.0.api_url,
            &config.0.dds_url,
            &config.0.client_id,
            &config.0.app_key.clone().unwrap(),
            &config.0.app_secret.clone().unwrap(),
        )
        .await
        .expect("Failed to create client");

        // Download all metadata for the domain
        let result = client
            .download_metadata(
                &config.1,
                &DownloadQuery {
                    ids: vec![],
                    name: None,
                    data_type: Some("test".to_string()),
                },
            )
            .await;
        assert!(
            result.is_ok(),
            "Failed to download domain metadata: {:?}",
            result.err()
        );
        let result = result.unwrap();
        assert!(result.len() > 0);
        for meta in result {
            assert!(!meta.id.is_empty());
            assert_eq!(meta.domain_id, config.1);
            assert!(!meta.name.is_empty());
            assert_eq!(meta.data_type, "test");
        }
    }

    #[tokio::test]
    async fn test_load_domain_with_oidc_access_token() {
        let config = get_config();
        // Assume we have a function to get a valid oidc_access_token for testing
        let oidc_access_token =
            std::env::var("AUTH_TEST_TOKEN").expect("AUTH_TEST_TOKEN env var not set");
        if oidc_access_token.is_empty() {
            eprintln!("Missing AUTH_TEST_TOKEN, skipping test");
            return;
        }

        let client =
            DiscoveryService::new(&config.0.api_url, &config.0.dds_url, &config.0.client_id);

        let domain = client
            .with_oidc_access_token(&oidc_access_token)
            .auth_domain(&config.1)
            .await;
        assert!(domain.is_ok(), "Failed to get domain: {:?}", domain.err());
        assert_eq!(domain.unwrap().domain.id, config.1);
    }

    #[tokio::test]
    async fn test_list_domains() {
        let config = get_config();
        let client = DomainClient::new_with_app_credential(
            &config.0.api_url,
            &config.0.dds_url,
            &config.0.client_id,
            &config.0.app_key.unwrap(),
            &config.0.app_secret.unwrap(),
        )
        .await
        .expect("Failed to create client");

        let org = std::env::var("TEST_ORGANIZATION").unwrap_or("own".to_string());
        let result = client.list_domains(&org).await.unwrap();
        assert!(result.len() > 0, "No domains found");
    }

    #[tokio::test]
    async fn test_submit_job_request_v1_with_invalid_processing_type() {
        let config = get_config();
        let client = DomainClient::new_with_user_credential(
            &config.0.api_url,
            &config.0.dds_url,
            &config.0.client_id,
            &config.0.email.unwrap(),
            &config.0.password.unwrap(),
            true,
        )
        .await
        .expect("Failed to create client");

        let mut job_request= JobRequest::default();
        job_request.processing_type = "invalid_processing_type".to_string();
        let res = client.submit_job_request_v1(&config.1, &job_request).await.expect_err("Failed to submit job request");
        assert_eq!(res.to_string(), "Failed to process domain. Status: 400 Bad Request - invalid processing type");
    }
}

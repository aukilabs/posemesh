use futures::channel::mpsc::Receiver;
use crate::domain_data::{
    DomainData, DomainDataMetadata, DownloadQuery, UploadDomainData, delete_by_id, download_by_id, download_metadata_v1, download_v1_stream, upload_v1
};

use crate::auth::TokenCache;
use crate::discovery::{DiscoveryService, DomainWithServer, DomainWithToken};
use crate::errors::DomainError;
pub use crate::reconstruction::JobRequest;
pub use crate::config;
pub use crate::auth;

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
    ) -> Result<Self, DomainError> {
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
    ) -> Result<Self, DomainError> {
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

    pub async fn download_domain_data_stream(
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

    pub async fn download_domain_data(
        &self,
        domain_id: &str,
        query: &DownloadQuery,
    ) -> Result<
        Vec<DomainData>,
        DomainError,
    > {
        use futures::StreamExt;
        let mut rx = self.download_domain_data_stream(domain_id, query).await?;

        let mut results = Vec::new();
        while let Some(result) = rx.next().await {
            results.push(result?);
        }
        Ok(results)
    }

    #[cfg(not(target_family = "wasm"))]
    pub async fn upload_domain_data_stream(
        &self,
        domain_id: &str,
        data: Receiver<UploadDomainData>,
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

    pub async fn create_domain(
        &self,
        name: &str,
        domain_server_id: Option<String>,
        domain_server_url: Option<String>,
        redirect_url: Option<String>,
    ) -> Result<DomainWithToken, DomainError> {
        self.discovery_client.create_domain(name, domain_server_id, domain_server_url, redirect_url).await
    }

    pub async fn delete_domain(
        &self,
        domain_id: &str,
    ) -> Result<(), DomainError> {
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        self.discovery_client.delete_domain(&domain.get_access_token(), domain_id).await
    }
}

#[cfg(not(target_family = "wasm"))]
#[cfg(test)]
mod tests {
    use crate::{auth::AuthClient, domain_data::{DomainAction, UploadDomainData}};

    use super::*;
    use futures::channel::mpsc;
    use tokio::spawn;

    fn get_config() -> (config::Config, String) {
        if std::path::Path::new("../.env.local").exists() {
            dotenvy::from_filename("../.env.local").ok();
        }
        dotenvy::dotenv().ok();
        let config = config::Config::from_env().unwrap();
        (config, std::env::var("DOMAIN_ID").unwrap())
    }

    async fn create_test_domain(config: &config::Config) -> Result<DomainWithToken, DomainError> {
        let client = DomainClient::new_with_user_credential(
            &config.api_url,
            &config.dds_url,
            &config.client_id,
            &config.email.clone().unwrap(),
            &config.password.clone().unwrap(),
            true,
        )
        .await
        .expect("Failed to create test client");
        client.create_domain(
            &format!("test_domain_{}", uuid::Uuid::new_v4()),
            None,
            Some(std::env::var("TEST_DOMAIN_SERVER_URL").unwrap()),
            None,
        )
        .await
    }

    async fn delete_test_domain(config: &config::Config, domain_id: &str) -> Result<(), DomainError> {
        let client = DomainClient::new_with_user_credential(
            &config.api_url,
            &config.dds_url,
            &config.client_id,
            &config.email.clone().unwrap(),
            &config.password.clone().unwrap(),
            true,
        )
        .await
        .expect("Failed to create test client");
        client.delete_domain(domain_id).await
    }

    async fn create_test_domain_data(config: &config::Config, domain_id: &str) -> Result<Vec<DomainDataMetadata>, DomainError> {
        let client = DomainClient::new_with_user_credential(
            &config.api_url,
            &config.dds_url,
            &config.client_id,
            &config.email.clone().unwrap(),
            &config.password.clone().unwrap(),
            true,
        )
        .await
        .expect("Failed to create test client");

        let data = vec![
            UploadDomainData {
                action: DomainAction::Create {
                    name: "to be deleted".to_string(),
                    data_type: "test".to_string(),
                },
                data: "{\"test\": \"test\"}".as_bytes().to_vec(),
            },
        ];
        client.upload_domain_data(domain_id, data).await
    }

    #[tokio::test]
    async fn get_organization_id() {
        let config = get_config();
        let mut client = AuthClient::new(
            &config.0.api_url,
            &config.0.client_id,
        );
        client.sign_in_with_app_credentials(&config.0.app_key.unwrap(), &config.0.app_secret.unwrap()).await.expect("Failed to sign in with app credentials");
        let token = client.get_dds_access_token(None).await.expect("Failed to get DDS access token");
        let claims = auth::parse_jwt(&token).expect("Failed to parse JWT");
        assert_ne!(claims.org.is_some(), false);
    }

    #[tokio::test]
    async fn test_download_domain_data_with_app_credential() {
        // Create a test client
        let config = get_config();
        let config = config.0.clone();
        let client = DomainClient::new_with_app_credential(
            &config.api_url,
            &config.dds_url,
            &config.client_id,
            &config.app_key.clone().unwrap(),
            &config.app_secret.clone().unwrap(),
        )
        .await
        .expect("Failed to create client");

        let domain = create_test_domain(&config).await.expect("Failed to create test domain");
        let domain_id = domain.domain.id.clone();

        let created = create_test_domain_data(&config, &domain_id).await.expect("Failed to create test domain data");
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].name, "to be deleted");
        assert_eq!(created[0].data_type, "test");

        // Create a test query
        let query = DownloadQuery {
            ids: vec![],
            name: None,
            data_type: Some("test".to_string()),
        };

        // Test the download function
        let result = client.download_domain_data(&domain_id, &query).await;

        assert!(result.is_ok(), "error message : {:?}", result.err());

        let results = result.unwrap();
        assert!(results.len() > 0);
        for result in results {
            assert_eq!(result.metadata.data_type, "test");
        }

        // Delete the domain
        delete_test_domain(&config, &domain_id).await.expect("Failed to delete test domain");
    }

    #[tokio::test]
    async fn test_upload_domain_data_with_user_credential() {
        use futures::SinkExt;
        let config = get_config();
        let client = DomainClient::new_with_user_credential(
            &config.0.api_url,
            &config.0.dds_url,
            &config.0.client_id,
            &config.0.email.clone().unwrap(),
            &config.0.password.clone().unwrap(),
            true,
        )
        .await
        .expect("Failed to create client");

        let domain = create_test_domain(&config.0).await.expect("Failed to create test domain");
        let domain_id = domain.domain.id.clone();

        let created = create_test_domain_data(&config.0, &domain_id).await.expect("Failed to create test domain data");
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].name, "to be deleted");
        assert_eq!(created[0].data_type, "test");

        let data = vec![
            UploadDomainData {
                action: DomainAction::Update {
                    id: created[0].id.clone(),
                },
                data: "{\"test\": \"test updated\"}".as_bytes().to_vec(),
            },
            UploadDomainData {
                action: DomainAction::Create {
                    name: "to be deleted2".to_string(),
                    data_type: "test".to_string(),
                },
                data: "{\"test\": \"test\"}".as_bytes().to_vec(),
            },
        ];
        let (mut tx, rx) = mpsc::channel(10);
        spawn(async move {
            for d in data {
                tx.send(d).await.unwrap();
            }
            tx.close().await.unwrap();
        });
        let result = client.upload_domain_data_stream(&domain_id, rx).await;
        assert!(result.is_ok(), "error message : {:?}", result.err());
        let created2 = result.unwrap();
        assert_eq!(created2.len(), 2);

        let ids = created2.iter().map(|d| d.id.clone()).collect::<Vec<String>>();
        assert_eq!(ids.len(), 2);
        // Create a test query
        let query = DownloadQuery {
            ids: ids,
            name: None,
            data_type: None,
        };

        // Test the download function
        let result = client.download_domain_data(&domain_id, &query).await;

        assert!(result.is_ok(), "error message : {:?}", result.err());

        let mut to_delete = None;
        let mut count = 0;
        let results = result.unwrap();
        for result in results {
            count += 1;
            if result.metadata.id == created[0].id {
                assert_eq!(result.data, b"{\"test\": \"test updated\"}");
                continue;
            } else {
                assert_eq!(result.data, b"{\"test\": \"test\"}");
            }
            to_delete = Some(result.metadata.id.clone());
        }
        assert_eq!(count, 2);
        assert_eq!(to_delete.is_some(), true);

        // Delete the one whose id is not "a8"
        let delete_result = client
            .delete_domain_data_by_id(&domain_id, &to_delete.unwrap())
            .await;
        assert!(
            delete_result.is_ok(),
            "Failed to delete data by id: {:?}",
            delete_result.err()
        );

        // Delete the domain
        delete_test_domain(&config.0, &domain_id).await.expect("Failed to delete test domain");
    }

    #[tokio::test]
    async fn test_download_domain_data_by_id() {
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

        let domain = create_test_domain(&config.0).await.expect("Failed to create test domain");
        let domain_id = domain.domain.id.clone();

        let created = create_test_domain_data(&config.0, &domain_id).await.expect("Failed to create test domain data");
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].name, "to be deleted");
        assert_eq!(created[0].data_type, "test");

        // Now test download by id
        let download_result = client
            .download_domain_data_by_id(&domain_id, &created[0].id)
            .await;

        assert!(
            download_result.is_ok(),
            "download by id failed: {:?}",
            download_result.err()
        );
        let downloaded_bytes = download_result.unwrap();
        assert_eq!(downloaded_bytes, b"{\"test\": \"test\"}".to_vec());

        // Delete the domain
        delete_test_domain(&config.0, &domain_id).await.expect("Failed to delete test domain");
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

        let domain = create_test_domain(&config.0).await.expect("Failed to create test domain");
        let domain_id = domain.domain.id.clone();

        let created = create_test_domain_data(&config.0, &domain_id).await.expect("Failed to create test domain data");
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].name, "to be deleted");
        assert_eq!(created[0].data_type, "test");

        // Download all metadata for the domain
        let result = client
            .download_metadata(
                &domain_id,
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
            assert_eq!(meta.domain_id, domain_id);
            assert!(!meta.name.is_empty());
            assert_eq!(meta.data_type, "test");
        }

        // Delete the domain
        delete_test_domain(&config.0, &domain_id).await.expect("Failed to delete test domain");
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
        assert_eq!(res.to_string(), "Auki response - status: 400 Bad Request, error: Failed to process domain. invalid processing type");
    }
}

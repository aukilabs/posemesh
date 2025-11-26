use std::sync::Arc;
use futures::StreamExt;
use crate::{discovery::DiscoveryService, domain_data::{DomainData, DownloadQuery, download_v1_stream}, errors::DomainError};
use crate::auth::TokenCache;

#[derive(Debug, Clone)]
pub struct DomainClient {
    discovery_client: DiscoveryService,
    pub client_id: String,
}


pub async fn new_with_app_credential(api_url: &str, dds_url: &str, client_id: &str, app_key: &str, app_secret: &str) -> Result<Arc<DomainClient>, DomainError> {
    let mut discovery = DiscoveryService::new(&api_url, &dds_url, &client_id);
    discovery.sign_in_as_auki_app(&app_key, &app_secret).await?;
    Ok(Arc::new(DomainClient {
        discovery_client: discovery,
        client_id: client_id.to_string(),
    }))
}

pub async fn new_with_user_credential(api_url: &str, dds_url: &str, client_id: &str, email: &str, password: &str, remember_password: bool) -> Result<Arc<DomainClient>, DomainError> {
    let mut discovery = DiscoveryService::new(&api_url, &dds_url, &client_id);
    discovery.sign_in_with_auki_account(&email, &password, remember_password).await?;
    Ok(Arc::new(DomainClient {
        discovery_client: discovery,
        client_id: client_id.to_string(),
    }))
}

impl DomainClient {
    pub fn new(api_url: &str, dds_url: &str, client_id: &str) -> Self {
        Self {
            discovery_client: DiscoveryService::new(&api_url, &dds_url, &client_id),
            client_id: client_id.to_string(),
        }
    }

    pub fn with_oidc_access_token(&self, token: &str) -> Arc<Self> {
        let discovery = self.discovery_client.with_oidc_access_token(token);
        Arc::new(Self {
            discovery_client: discovery,
            client_id: self.client_id.clone(),
        })
    }

    pub async fn download_domain_data(
        &self,
        domain_id: &str,
        query: &DownloadQuery,
    ) -> Result<Vec<DomainData>, DomainError> {
        let domain = self.discovery_client.auth_domain(domain_id).await?;
        let mut rx = download_v1_stream(
            &domain.domain.domain_server.url,
            &self.client_id,
            &domain.get_access_token(),
            domain_id,
            query,
        )
        .await?;
        
        // Convert Receiver to Vec by collecting all items
        let mut results = Vec::new();
        while let Some(result) = rx.next().await {
            results.push(result?);
        }
        Ok(results)
    }
}

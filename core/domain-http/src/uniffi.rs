use std::sync::Arc;
use posemesh_runtime::get_runtime;
use crate::{discovery::DomainWithServer, domain_data::{DomainData, DomainDataMetadata, DownloadQuery, UploadDomainData}, errors::DomainError};
use crate::domain_client::DomainClient as r_DomainClient;

#[derive(Debug, Clone)]
pub struct DomainClient(r_DomainClient);

pub fn new_with_app_credential(api_url: &str, dds_url: &str, client_id: &str, app_key: &str, app_secret: &str) -> Result<Arc<DomainClient>, DomainError> {
    // posemesh_runtime::with_runtime(|| async move {
    //     let dc = r_DomainClient::new_with_app_credential(api_url, dds_url, client_id, app_key, app_secret).await?;
    //     Ok(Arc::new(DomainClient(dc)))
    // }).await

    get_runtime().block_on(async move {
        let dc = r_DomainClient::new_with_app_credential(api_url, dds_url, client_id, app_key, app_secret).await?;
        Ok(Arc::new(DomainClient(dc)))
    })
}

pub fn new_with_user_credential(api_url: &str, dds_url: &str, client_id: &str, email: &str, password: &str, remember_password: bool) -> Result<Arc<DomainClient>, DomainError> {
    // posemesh_runtime::with_runtime(|| async move {
    //     let dc = r_DomainClient::new_with_user_credential(api_url, dds_url, client_id, email, password, remember_password).await?;
    //     Ok(Arc::new(DomainClient(dc)))
    // }).await

    get_runtime().block_on(async move {
        let dc = r_DomainClient::new_with_user_credential(api_url, dds_url, client_id, email, password, remember_password).await?;
        Ok(Arc::new(DomainClient(dc)))
    })
}

impl DomainClient {
    pub fn new(api_url: &str, dds_url: &str, client_id: &str) -> Self {
        Self(r_DomainClient::new(api_url, dds_url, client_id))
    }

    pub fn with_oidc_access_token(&self, token: &str) -> Arc<Self> {
        let dc = self.0.with_oidc_access_token(token);
        Arc::new(DomainClient(dc))
    }

    pub fn download_domain_data(
        &self,
        domain_id: &str,
        query: &DownloadQuery,
    ) -> Result<Vec<DomainData>, DomainError> {
        // posemesh_runtime::with_runtime(|| async move {
        //     self.0.download_domain_data(domain_id, query).await
        // }).await

        get_runtime().block_on(async move {
            self.0.download_domain_data(domain_id, query).await
        })
    }

    pub fn upload_domain_data(
        &self,
        domain_id: &str,
        data: Vec<UploadDomainData>,
    ) -> Result<Vec<DomainDataMetadata>, DomainError> {
        let res = get_runtime().block_on(async move {
            self.0.upload_domain_data(domain_id, data).await
        })?;
        Ok(res)
    }

    pub fn create_domain(
        &self,
        name: &str,
        domain_server_id: Option<String>,
        domain_server_url: Option<String>,
        redirect_url: Option<String>,
    ) -> Result<DomainWithServer, DomainError> {
        let res = get_runtime().block_on(async move {
            let res = self.0.create_domain(name, domain_server_id, domain_server_url, redirect_url).await?;
            Ok(res.domain) as Result<DomainWithServer, DomainError>
        })?;
        Ok(res)
    }

    pub fn delete_domain(
        &self,
        domain_id: &str,
    ) -> Result<(), DomainError> {
        let res = get_runtime().block_on(async move {
            self.0.delete_domain(domain_id).await
        })?;
        Ok(res)
    }
}

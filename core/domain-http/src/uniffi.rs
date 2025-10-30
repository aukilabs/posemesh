use uniffi::{constructor, export, Object};

use crate::DomainClient as r_DomainClient;

pub struct DomainClient {
    domain_client: r_DomainClient,
}

#[export]
impl DomainClient {
    #[constructor]
    pub fn new(api_url: String, dds_url: String, client_id: String) -> Self {
        Self {
            domain_client: r_DomainClient::new(&api_url, &dds_url, &client_id),
        }
    }

    // #[constructor]
    // pub fn sign_in_with_app_credential(&self, api_url: &str, dds_url: &str, client_id: &str, app_key: &str, app_secret: &str) -> Self {
    //     get_runtime().block_on(async move {
    //         r_DomainClient::new_with_app_credential(api_url, dds_url, client_id, app_key, app_secret).await.unwrap()
    //     })
    // }

    // #[constructor]
    // pub fn sign_in_with_user_credential(&self, api_url: &str, dds_url: &str, client_id: &str, email: &str, password: &str, remember_password: bool) -> Self {
    //     Self {
    //         domain_client: r_DomainClient::new_with_user_credential(api_url, dds_url, client_id, email, password, remember_password).unwrap(),
    //     }
    // }
}

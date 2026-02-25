pub mod auth;
pub mod config;
pub mod discovery;
pub mod domain_client;
pub mod domain_data;
pub mod errors;
pub mod reconstruction;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(target_family = "wasm")]
pub mod wasm;

#[cfg(feature = "uniffi")]
use crate::{
    discovery::{DomainServer, DomainWithServer, ListDomainsResponse},
    domain_client::ListDomainsQuery,
    domain_data::{DomainAction, DomainData, DomainDataMetadata, DownloadQuery, UploadDomainData},
    errors::DomainError,
};

#[cfg(feature = "uniffi")]
pub mod uniffi;

#[cfg(feature = "uniffi")]
use crate::uniffi::{DomainClient, new_with_app_credential, new_with_user_credential};

#[cfg(feature = "uniffi")]
::uniffi::include_scaffolding!("domain-client");

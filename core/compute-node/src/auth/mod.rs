pub mod robot;
pub mod siwe;
pub mod siwe_after_registration;
pub mod token_manager;

use async_trait::async_trait;

use crate::dds::p2p::PeerBindingClient;

pub use robot::{RobotHandle, RobotMachineAuth};
pub use siwe::{AccessBundle, SiweError};
pub use siwe_after_registration::{SiweAfterRegistration, SiweHandle};
pub use token_manager::{
    AccessAuthenticator, SystemClock, TokenManager, TokenManagerConfig, TokenManagerError,
    TokenProvider, TokenProviderError,
};

pub(crate) struct PeerBoundAuthenticator<A> {
    base: A,
    binding: Option<PeerBindingClient>,
}

impl<A> PeerBoundAuthenticator<A> {
    pub(crate) fn new(base: A, binding: Option<PeerBindingClient>) -> Self {
        Self { base, binding }
    }
}

#[async_trait]
impl<A> AccessAuthenticator for PeerBoundAuthenticator<A>
where
    A: AccessAuthenticator,
{
    async fn login(&self) -> std::result::Result<AccessBundle, SiweError> {
        let base = self.base.login().await?;
        match &self.binding {
            Some(binding) => binding
                .bind(&base)
                .await
                .map_err(|error| SiweError::PeerBinding(error.to_string())),
            None => Ok(base),
        }
    }
}

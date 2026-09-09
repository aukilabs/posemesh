//! Compatibility adapters for SDK machine peer auth and runner-owned renewal.

use std::{sync::Arc, time::Duration};

use auki_auth::machine::p2p as machine_p2p;
use auki_p2p::{DdsTokenVerifier, PeerId, PeerIdentityProof};
use auki_sdk::{ExternalAuthorityControl, ExternalAuthorityUpdate};
use chrono::{DateTime, Utc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::warn;
use url::Url;
use uuid::Uuid;

use crate::auth::{token_manager::TokenProvider, AccessBundle};

pub use machine_p2p::{DdsP2pError, Result};

const REFRESH_RETRY_DELAY: Duration = Duration::from_secs(30);
const REFRESH_SAFETY_RATIO: f64 = 0.75;

#[derive(Clone)]
pub struct DdsP2pClient {
    inner: machine_p2p::DdsP2pClient,
}

impl DdsP2pClient {
    pub fn new(base_url: Url, timeout: Duration) -> Result<Self> {
        Ok(Self {
            inner: machine_p2p::DdsP2pClient::new(base_url, timeout)?,
        })
    }

    pub async fn token_verifier(&self) -> Result<DdsTokenVerifier> {
        self.inner.token_verifier().await
    }

    pub async fn external_authority_update(
        &self,
        identity: &PeerIdentityProof,
        domain_id: Uuid,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<ExternalAuthorityUpdate> {
        let material = self
            .inner
            .authority_material(identity, domain_id, token, expires_at)
            .await?;
        Ok(ExternalAuthorityUpdate::new(
            material.domain_id,
            material.peer_id,
            material.verification_keys,
            material.credential,
            material.expires_at,
        ))
    }

    async fn robot_p2p_token(&self, machine_token: &str) -> Result<machine_p2p::RobotP2pToken> {
        self.inner.robot_p2p_token(machine_token).await
    }
}

#[derive(Clone)]
pub struct PeerBindingClient {
    inner: machine_p2p::PeerBindingClient,
}

impl PeerBindingClient {
    pub fn new(dds: DdsP2pClient, identity: PeerIdentityProof) -> Self {
        Self {
            inner: machine_p2p::PeerBindingClient::new(dds.inner, identity),
        }
    }

    pub fn peer_id(&self) -> PeerId {
        self.inner.peer_id()
    }

    pub async fn bind(&self, base: &AccessBundle) -> Result<AccessBundle> {
        self.inner.bind(base).await
    }
}

/// Robot authority source used before and after facade startup.
///
/// It owns no networking runtime. Every refresh obtains the current machine
/// bearer, exchanges it for a peer-bound DDS credential, and carries the
/// current verification-key set in one atomic facade update.
#[derive(Clone)]
pub struct RobotP2pAuthoritySource {
    dds: DdsP2pClient,
    machine_auth: Arc<dyn TokenProvider>,
    identity: PeerIdentityProof,
}

pub struct PreparedRobotP2pAuthority {
    domain_id: Uuid,
    expires_at: DateTime<Utc>,
    update: ExternalAuthorityUpdate,
}

impl RobotP2pAuthoritySource {
    pub fn new(
        dds: DdsP2pClient,
        machine_auth: Arc<dyn TokenProvider>,
        identity: PeerIdentityProof,
    ) -> Self {
        Self {
            dds,
            machine_auth,
            identity,
        }
    }

    pub async fn prepare(&self) -> Result<PreparedRobotP2pAuthority> {
        self.prepare_for_domain(None).await
    }

    async fn prepare_for_domain(
        &self,
        expected_domain_id: Option<Uuid>,
    ) -> Result<PreparedRobotP2pAuthority> {
        let machine_token = self
            .machine_auth
            .bearer()
            .await
            .map_err(DdsP2pError::MachineToken)?;
        let token = self.dds.robot_p2p_token(&machine_token).await?;
        if expected_domain_id.is_some_and(|expected| expected != token.domain_id) {
            return Err(DdsP2pError::CredentialDomainMismatch);
        }
        let update = self
            .dds
            .external_authority_update(
                &self.identity,
                token.domain_id,
                &token.token,
                token.expires_at,
            )
            .await?;
        Ok(PreparedRobotP2pAuthority {
            domain_id: token.domain_id,
            expires_at: token.expires_at,
            update,
        })
    }
}

impl PreparedRobotP2pAuthority {
    pub fn domain_id(&self) -> Uuid {
        self.domain_id
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub fn into_update(self) -> ExternalAuthorityUpdate {
        self.update
    }
}

/// Drives periodic Robot authority renewal and facade-requested relay refresh.
pub struct RobotP2pAuthorityDriver {
    stop: CancellationToken,
    task: Option<JoinHandle<()>>,
}

impl RobotP2pAuthorityDriver {
    pub fn start(
        source: RobotP2pAuthoritySource,
        domain_id: Uuid,
        initial_expires_at: DateTime<Utc>,
        control: ExternalAuthorityControl,
        shutdown: &CancellationToken,
    ) -> Result<Self> {
        let initial_delay = authority_refresh_delay(initial_expires_at)?;
        let stop = shutdown.child_token();
        let task_stop = stop.clone();
        let task = tokio::spawn(async move {
            let mut delay = initial_delay;
            loop {
                let refresh_requested = tokio::select! {
                    _ = task_stop.cancelled() => break,
                    _ = tokio::time::sleep(delay) => false,
                    request = control.next_refresh_request() => {
                        if request.is_none() {
                            break;
                        }
                        true
                    }
                };

                let prepared = tokio::select! {
                    _ = task_stop.cancelled() => break,
                    result = source.prepare_for_domain(Some(domain_id)) => result,
                };
                match prepared {
                    Ok(prepared) => {
                        let expires_at = prepared.expires_at();
                        match control.replace(prepared.into_update()).await {
                            Ok(_) => match authority_refresh_delay(expires_at) {
                                Ok(next) => delay = next,
                                Err(error) => {
                                    warn!(error = %error, "DDS Robot P2P authority expiration is invalid");
                                    delay = REFRESH_RETRY_DELAY;
                                }
                            },
                            Err(error) => {
                                warn!(error = %error, refresh_requested, "Auki peer rejected the refreshed Robot authority");
                                delay = REFRESH_RETRY_DELAY;
                            }
                        }
                    }
                    Err(error) => {
                        warn!(error = %error, refresh_requested, "DDS Robot P2P authority refresh failed");
                        delay = REFRESH_RETRY_DELAY;
                    }
                }
            }
        });
        Ok(Self {
            stop,
            task: Some(task),
        })
    }

    pub async fn shutdown(mut self) {
        self.stop.cancel();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

fn authority_refresh_delay(expires_at: DateTime<Utc>) -> Result<Duration> {
    let remaining = expires_at
        .signed_duration_since(Utc::now())
        .to_std()
        .map_err(|_| DdsP2pError::InvalidExpiration)?;
    Ok(remaining
        .mul_f64(REFRESH_SAFETY_RATIO)
        .max(Duration::from_secs(1)))
}

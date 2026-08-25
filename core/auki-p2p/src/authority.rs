use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{Error, Node, P2PAccessClaims, PeerId, PeerRole};

/// Fail-closed errors for the process-local authenticated P2P authority.
#[derive(Debug, thiserror::Error)]
pub enum P2pCredentialError {
    #[error("DDS P2P access token is invalid")]
    InvalidAccessToken(#[source] Error),
    #[error("DDS P2P access-token expiration is invalid")]
    InvalidExpiration,
    #[error("P2P token and expiration must be provided together")]
    IncompleteCredential,
    #[error("no current DDS P2P credential is installed")]
    MissingCredential,
    #[error("the current DDS P2P credential has expired")]
    ExpiredCredential,
    #[error("the current DDS P2P credential has the wrong local role")]
    CredentialRoleMismatch,
    #[error("the current DDS P2P credential does not authorize the required Domain")]
    CredentialDomainMismatch,
}

pub type P2pCredentialResult<T> = std::result::Result<T, P2pCredentialError>;

/// Shared local authority for every application protocol hosted by one Node.
///
/// Credential acquisition remains the host application's responsibility. This
/// type validates, installs, snapshots, and clears the one peer-bound token
/// used by the underlying authenticated transport.
#[derive(Clone)]
pub struct P2pCredentialStore {
    node: Node,
    current: Arc<Mutex<Option<P2PAccessClaims>>>,
}

impl P2pCredentialStore {
    pub fn new(node: Node) -> Self {
        Self {
            node,
            current: Arc::new(Mutex::new(None)),
        }
    }

    pub fn peer_id(&self) -> PeerId {
        self.node.peer_id()
    }

    pub async fn install(
        &self,
        token: impl Into<String>,
        expected_expires_at: DateTime<Utc>,
    ) -> P2pCredentialResult<P2PAccessClaims> {
        if expected_expires_at <= Utc::now() {
            return Err(P2pCredentialError::InvalidExpiration);
        }
        let mut current = self.current.lock().await;
        let claims = self
            .node
            .install_token(token)
            .await
            .map_err(P2pCredentialError::InvalidAccessToken)?;
        let claim_exp = i64::try_from(claims.exp)
            .ok()
            .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
            .ok_or(P2pCredentialError::InvalidExpiration)?;
        if claim_exp != expected_expires_at {
            self.node.clear_token().await;
            *current = None;
            return Err(P2pCredentialError::InvalidExpiration);
        }
        *current = Some(claims.clone());
        Ok(claims)
    }

    pub async fn install_optional(
        &self,
        token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> P2pCredentialResult<Option<P2PAccessClaims>> {
        match (token, expires_at) {
            (Some(token), Some(expires_at)) => {
                self.install(token.to_string(), expires_at).await.map(Some)
            }
            (None, None) => Ok(None),
            _ => Err(P2pCredentialError::IncompleteCredential),
        }
    }

    pub async fn require(
        &self,
        role: PeerRole,
        domain_id: Uuid,
    ) -> P2pCredentialResult<P2PAccessClaims> {
        let claims = self
            .current
            .lock()
            .await
            .clone()
            .ok_or(P2pCredentialError::MissingCredential)?;
        let now = Utc::now().timestamp();
        let expiration =
            i64::try_from(claims.exp).map_err(|_| P2pCredentialError::InvalidExpiration)?;
        if expiration <= now {
            return Err(P2pCredentialError::ExpiredCredential);
        }
        if claims.peer_type != role {
            return Err(P2pCredentialError::CredentialRoleMismatch);
        }
        if !claims
            .domain_ids
            .iter()
            .filter_map(|candidate| Uuid::parse_str(candidate).ok())
            .any(|candidate| candidate == domain_id)
        {
            return Err(P2pCredentialError::CredentialDomainMismatch);
        }
        Ok(claims)
    }

    pub async fn current_claims(&self) -> Option<P2PAccessClaims> {
        self.current.lock().await.clone()
    }

    pub async fn clear(&self) {
        let mut current = self.current.lock().await;
        self.node.clear_token().await;
        *current = None;
    }
}

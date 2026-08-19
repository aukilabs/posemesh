use std::{collections::HashSet, str::FromStr, sync::Arc, time::Duration};

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{Error, Result};

pub const P2P_TOKEN_TYPE: &str = "p2p-access";
pub const P2P_TOKEN_ISSUER: &str = "dds";
pub const P2P_TOKEN_AUDIENCE: &str = "auki-p2p";
pub const P2P_TOKEN_SCOPE: &str = "domain-data:r";
pub const P2P_TOKEN_TTL: Duration = Duration::from_secs(30 * 60);
pub const DOMAIN_SERVER_MAX_DOMAINS: usize = 25;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerRole {
    Robot,
    Compute,
    DomainServer,
}

impl std::fmt::Display for PeerRole {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Robot => formatter.write_str("robot"),
            Self::Compute => formatter.write_str("compute"),
            Self::DomainServer => formatter.write_str("domain_server"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct P2PAccessClaims {
    #[serde(rename = "type")]
    pub token_type: String,
    pub iss: String,
    pub aud: Vec<String>,
    pub sub: String,
    pub peer_type: PeerRole,
    pub peer_id: String,
    pub domain_ids: Vec<String>,
    pub scopes: Vec<String>,
    pub iat: u64,
    pub exp: u64,
}

#[derive(Clone)]
pub struct DdsTokenVerifier {
    key: Arc<DecodingKey>,
}

impl DdsTokenVerifier {
    /// The host supplies the DDS ES256 verification key; obtaining and
    /// refreshing that key remains outside this transport crate.
    pub fn from_es256_pem(public_key_pem: &[u8]) -> Result<Self> {
        Ok(Self {
            key: Arc::new(DecodingKey::from_ec_pem(public_key_pem)?),
        })
    }

    pub fn verify(&self, token: &str) -> Result<P2PAccessClaims> {
        let mut validation = Validation::new(Algorithm::ES256);
        validation.leeway = 0;
        validation.set_audience(&[P2P_TOKEN_AUDIENCE]);
        validation.set_issuer(&[P2P_TOKEN_ISSUER]);
        validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub"]);

        let claims = decode::<P2PAccessClaims>(token, &self.key, &validation)
            .map_err(Error::TokenVerification)?
            .claims;
        validate_profile(&claims)?;
        Ok(claims)
    }
}

fn validate_profile(claims: &P2PAccessClaims) -> Result<()> {
    if claims.token_type != P2P_TOKEN_TYPE {
        return Err(Error::InvalidToken("unexpected token type".into()));
    }
    if claims.iss != P2P_TOKEN_ISSUER {
        return Err(Error::InvalidToken("unexpected issuer".into()));
    }
    if claims.aud != [P2P_TOKEN_AUDIENCE] {
        return Err(Error::InvalidToken(
            "audience must be exactly [auki-p2p]".into(),
        ));
    }
    Uuid::parse_str(&claims.sub)
        .map_err(|_| Error::InvalidToken("subject must be a machine UUID".into()))?;
    PeerId::from_str(&claims.peer_id)
        .map_err(|_| Error::InvalidToken("peer_id must be a libp2p Peer ID".into()))?;
    if claims.scopes != [P2P_TOKEN_SCOPE] {
        return Err(Error::InvalidToken(
            "scopes must be exactly [domain-data:r]".into(),
        ));
    }
    if claims.exp.checked_sub(claims.iat) != Some(P2P_TOKEN_TTL.as_secs()) {
        return Err(Error::InvalidToken(
            "expiration must be exactly 30 minutes after issued-at".into(),
        ));
    }

    let valid_count = match claims.peer_type {
        PeerRole::Robot | PeerRole::Compute => claims.domain_ids.len() == 1,
        PeerRole::DomainServer => {
            (1..=DOMAIN_SERVER_MAX_DOMAINS).contains(&claims.domain_ids.len())
        }
    };
    if !valid_count {
        return Err(Error::InvalidToken(
            "invalid Domain count for peer role".into(),
        ));
    }

    let mut unique_domains = HashSet::with_capacity(claims.domain_ids.len());
    for domain_id in &claims.domain_ids {
        let parsed = Uuid::parse_str(domain_id)
            .map_err(|_| Error::InvalidToken("domain_ids must contain UUIDs".into()))?;
        if !unique_domains.insert(parsed) {
            return Err(Error::InvalidToken("domain_ids must be unique".into()));
        }
    }
    Ok(())
}

#[derive(Clone, Default)]
pub(crate) struct TokenStore {
    current: Arc<RwLock<Option<String>>>,
}

impl TokenStore {
    pub async fn install(
        &self,
        token: String,
        verifier: &DdsTokenVerifier,
        local_peer_id: PeerId,
    ) -> Result<P2PAccessClaims> {
        let claims = verifier.verify(&token)?;
        ensure_token_peer(&claims, local_peer_id)?;
        *self.current.write().await = Some(token);
        Ok(claims)
    }

    pub async fn clear(&self) {
        *self.current.write().await = None;
    }

    pub async fn snapshot(&self) -> Option<String> {
        self.current.read().await.clone()
    }
}

pub(crate) fn ensure_token_peer(claims: &P2PAccessClaims, noise_peer_id: PeerId) -> Result<()> {
    let token_peer_id = PeerId::from_str(&claims.peer_id)
        .map_err(|_| Error::InvalidToken("peer_id must be a libp2p Peer ID".into()))?;
    if token_peer_id != noise_peer_id {
        return Err(Error::PeerIdMismatch {
            token_peer_id: token_peer_id.to_string(),
            noise_peer_id: noise_peer_id.to_string(),
        });
    }
    Ok(())
}

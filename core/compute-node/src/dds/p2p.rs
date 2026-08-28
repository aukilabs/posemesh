use std::{collections::HashSet, sync::Arc, time::Duration};

use async_trait::async_trait;
pub use auki_p2p::DomainAuthority;
use auki_p2p::{
    DdsTokenVerifier, DdsVerificationKeys, Identity, Multiaddr, Node, P2PAccessClaims, PeerId,
    PeerIdentityProof, SignedP2pCredential, DDS_PREVIOUS_KEY_MIN_OVERLAP,
    DDS_VERIFICATION_KEY_MAX_BYTES,
};
use auki_sdk::ExternalAuthorityUpdate;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use p256::pkcs8::{DecodePublicKey, EncodePublicKey};
use reqwest::{
    header::{ACCEPT, CACHE_CONTROL, CONTENT_TYPE},
    redirect::Policy,
    Client, StatusCode,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{
    sync::{Mutex, OnceCell, RwLock},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use url::Url;
use uuid::Uuid;

use crate::auth::{AccessBundle, TokenProvider, TokenProviderError};

const CHALLENGE_PATH: &str = "/internal/v1/auth/p2p/challenge";
const VERIFY_PATH: &str = "/internal/v1/auth/p2p/verify";
const ROBOT_TOKEN_PATH: &str = "/internal/v1/auth/robot/p2p-token";
const VERIFICATION_KEYS_PATH: &str = "/service/p2p-verification-keys";
const REFRESH_RETRY_DELAY: Duration = Duration::from_secs(30);
const REFRESH_SAFETY_RATIO: f64 = 0.75;
const VERIFICATION_KEY_REFRESH_INTERVAL: Duration = Duration::from_secs(2 * 60);
const VERIFICATION_KEY_REFRESH_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum DdsP2pError {
    #[error("failed to build DDS P2P HTTP client")]
    Client(#[source] reqwest::Error),
    #[error("DDS P2P request failed")]
    Request(#[source] reqwest::Error),
    #[error("DDS P2P endpoint returned status {0}")]
    UpstreamStatus(StatusCode),
    #[error("DDS P2P response missing field '{0}'")]
    MissingField(&'static str),
    #[error("DDS P2P challenge encoding is invalid")]
    InvalidChallengeEncoding(#[source] base64::DecodeError),
    #[error("DDS P2P identity proof failed")]
    IdentityProof(#[source] auki_p2p::Error),
    #[error("DDS P2P response returned a different Peer ID")]
    PeerIdMismatch,
    #[error("DDS P2P verification public key is invalid")]
    InvalidPublicKey(#[source] auki_p2p::Error),
    #[error("DDS P2P verification public key response is too large")]
    VerificationKeyResponseTooLarge,
    #[error("DDS P2P verification-key response is invalid: {0}")]
    InvalidVerificationKeyResponse(&'static str),
    #[error("Auki P2P node operation failed")]
    Node(#[source] auki_p2p::Error),
    #[error("machine token is malformed")]
    MalformedMachineToken,
    #[error("Robot machine token has no assigned Domain")]
    MissingRobotAssignment,
    #[error("Robot machine token is unavailable")]
    MachineToken(#[source] TokenProviderError),
    #[error("DDS P2P access token is invalid")]
    InvalidAccessToken(#[source] auki_p2p::Error),
    #[error("local P2P authority rejected the credential")]
    Credential(#[from] auki_p2p::P2pCredentialError),
    #[error("DDS P2P access-token expiration is invalid")]
    InvalidExpiration,
    #[error("DDS P2P token and expiration must be provided together")]
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

pub type Result<T> = std::result::Result<T, DdsP2pError>;

#[derive(Clone)]
pub struct DdsP2pClient {
    base_url: Url,
    http: Client,
    initial_verification: Arc<OnceCell<InitialVerification>>,
}

struct InitialVerification {
    verifier: DdsTokenVerifier,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationKeysResponse {
    version: u8,
    generation: u64,
    previous_key_overlap_seconds: u64,
    keys: Vec<VerificationKeyResponse>,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum VerificationKeyStatus {
    Current,
    Previous,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationKeyResponse {
    id: String,
    status: VerificationKeyStatus,
    signing_method: String,
    public_key: String,
}

fn convert_verification_keys(response: VerificationKeysResponse) -> Result<DdsVerificationKeys> {
    if response.version != 1 || response.generation == 0 {
        return Err(DdsP2pError::InvalidVerificationKeyResponse(
            "unsupported key-set version or zero generation",
        ));
    }
    if response.previous_key_overlap_seconds < DDS_PREVIOUS_KEY_MIN_OVERLAP.as_secs() {
        return Err(DdsP2pError::InvalidVerificationKeyResponse(
            "previous-key overlap is below the SDK safety window",
        ));
    }
    if !(1..=2).contains(&response.keys.len())
        || response.keys[0].status != VerificationKeyStatus::Current
        || response
            .keys
            .get(1)
            .is_some_and(|key| key.status != VerificationKeyStatus::Previous)
    {
        return Err(DdsP2pError::InvalidVerificationKeyResponse(
            "keys must contain current first and at most one previous key",
        ));
    }

    let mut parsed = Vec::with_capacity(response.keys.len());
    let mut ids = HashSet::with_capacity(response.keys.len());
    for key in response.keys {
        if key.id.len() != 64
            || !key
                .id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || !ids.insert(key.id.clone())
            || key.signing_method != "ES256"
            || key.public_key.is_empty()
            || key.public_key.len() > DDS_VERIFICATION_KEY_MAX_BYTES
        {
            return Err(DdsP2pError::InvalidVerificationKeyResponse(
                "verification-key metadata or PEM is invalid",
            ));
        }
        if verification_key_id(&key.public_key)? != key.id {
            return Err(DdsP2pError::InvalidVerificationKeyResponse(
                "verification-key id does not match its canonical PKIX fingerprint",
            ));
        }
        parsed.push(key.public_key.into_bytes());
    }

    let current = parsed.remove(0);
    let previous = parsed.pop();
    Ok(DdsVerificationKeys::new(
        response.generation,
        current,
        previous,
    ))
}

fn verification_key_id(public_key_pem: &str) -> Result<String> {
    let public_key = p256::PublicKey::from_public_key_pem(public_key_pem).map_err(|_| {
        DdsP2pError::InvalidVerificationKeyResponse(
            "verification key must contain a P-256 public key",
        )
    })?;
    let canonical_der = public_key.to_public_key_der().map_err(|_| {
        DdsP2pError::InvalidVerificationKeyResponse(
            "verification key could not be encoded as canonical PKIX DER",
        )
    })?;
    Ok(hex::encode(Sha256::digest(canonical_der.as_bytes())))
}

impl DdsP2pClient {
    pub fn new(base_url: Url, timeout: Duration) -> Result<Self> {
        let http = Client::builder()
            .use_rustls_tls()
            .no_proxy()
            .redirect(Policy::none())
            .timeout(timeout)
            .build()
            .map_err(DdsP2pError::Client)?;
        Ok(Self {
            base_url,
            http,
            initial_verification: Arc::new(OnceCell::new()),
        })
    }

    pub async fn token_verifier(&self) -> Result<DdsTokenVerifier> {
        Ok(self.initial_verification().await?.verifier.clone())
    }

    async fn initial_verification(&self) -> Result<&InitialVerification> {
        self.initial_verification
            .get_or_try_init(|| async {
                let keys = self.fetch_verification_keys().await?;
                let verifier =
                    DdsTokenVerifier::from_keys(keys).map_err(DdsP2pError::InvalidPublicKey)?;
                Ok(InitialVerification { verifier })
            })
            .await
    }

    async fn fetch_verification_keys(&self) -> Result<DdsVerificationKeys> {
        let response = self
            .http
            .get(self.endpoint(VERIFICATION_KEYS_PATH))
            .header(ACCEPT, "application/json")
            .header(CACHE_CONTROL, "no-cache")
            .send()
            .await
            .map_err(DdsP2pError::Request)?;
        ensure_success(&response)?;
        let is_json = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"));
        if !is_json {
            return Err(DdsP2pError::InvalidVerificationKeyResponse(
                "Content-Type must be application/json",
            ));
        }
        if response
            .content_length()
            .is_some_and(|length| length > DDS_VERIFICATION_KEY_MAX_BYTES as u64)
        {
            return Err(DdsP2pError::VerificationKeyResponseTooLarge);
        }

        let mut encoded = Vec::new();
        let mut body = response.bytes_stream();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(DdsP2pError::Request)?;
            if encoded.len().saturating_add(chunk.len()) > DDS_VERIFICATION_KEY_MAX_BYTES {
                return Err(DdsP2pError::VerificationKeyResponseTooLarge);
            }
            encoded.extend_from_slice(&chunk);
        }
        let response: VerificationKeysResponse =
            serde_json::from_slice(&encoded).map_err(|_| {
                DdsP2pError::InvalidVerificationKeyResponse("JSON does not match the V1 contract")
            })?;
        convert_verification_keys(response)
    }

    async fn refresh_verification_keys(&self, authority: &DomainAuthority) -> Result<()> {
        authority
            .install_verification_keys(self.fetch_verification_keys().await?)
            .await
            .map_err(DdsP2pError::Node)
    }

    async fn create_challenge(
        &self,
        machine_token: &str,
        identity: &PeerIdentityProof,
    ) -> Result<ChallengeResponse> {
        let request = ChallengeRequest {
            peer_id: identity.peer_id().to_string(),
            public_key: URL_SAFE_NO_PAD.encode(identity.public_key_protobuf()),
        };
        let response = self
            .http
            .post(self.endpoint(CHALLENGE_PATH))
            .bearer_auth(machine_token)
            .json(&request)
            .send()
            .await
            .map_err(DdsP2pError::Request)?;
        ensure_success(&response)?;
        let response: ChallengeResponse = response.json().await.map_err(DdsP2pError::Request)?;
        if response.challenge_id.trim().is_empty() {
            return Err(DdsP2pError::MissingField("challenge_id"));
        }
        if response.challenge.trim().is_empty() {
            return Err(DdsP2pError::MissingField("challenge"));
        }
        if response.expires_at <= Utc::now() {
            return Err(DdsP2pError::InvalidExpiration);
        }
        Ok(response)
    }

    async fn verify_challenge(
        &self,
        machine_token: &str,
        identity: &PeerIdentityProof,
        challenge: ChallengeResponse,
    ) -> Result<AccessBundle> {
        let challenge_bytes = URL_SAFE_NO_PAD
            .decode(challenge.challenge)
            .map_err(DdsP2pError::InvalidChallengeEncoding)?;
        let signature = identity
            .sign_challenge(&challenge_bytes)
            .map_err(DdsP2pError::IdentityProof)?;
        let request = VerifyRequest {
            challenge_id: challenge.challenge_id,
            signature: URL_SAFE_NO_PAD.encode(signature),
        };
        let response = self
            .http
            .post(self.endpoint(VERIFY_PATH))
            .bearer_auth(machine_token)
            .json(&request)
            .send()
            .await
            .map_err(DdsP2pError::Request)?;
        ensure_success(&response)?;
        let response: VerifyResponse = response.json().await.map_err(DdsP2pError::Request)?;
        if response.peer_id != identity.peer_id().to_string() {
            return Err(DdsP2pError::PeerIdMismatch);
        }
        if response.access_token.is_empty() {
            return Err(DdsP2pError::MissingField("access_token"));
        }
        if response.access_expires_at <= Utc::now() {
            return Err(DdsP2pError::InvalidExpiration);
        }
        Ok(AccessBundle::new(
            response.access_token,
            response.access_expires_at,
        ))
    }

    /// Build the complete fixed-Domain authority envelope consumed by
    /// `AukiPeer::start_external` and `ExternalAuthorityControl::replace`.
    pub async fn external_authority_update(
        &self,
        identity: &PeerIdentityProof,
        domain_id: Uuid,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<ExternalAuthorityUpdate> {
        if expires_at <= Utc::now() {
            return Err(DdsP2pError::InvalidExpiration);
        }
        let verification_keys = self.fetch_verification_keys().await?;
        let credential =
            SignedP2pCredential::new(token.to_owned()).map_err(DdsP2pError::InvalidAccessToken)?;
        Ok(ExternalAuthorityUpdate::new(
            domain_id,
            identity.peer_id(),
            verification_keys,
            credential,
            expires_at,
        ))
    }

    async fn robot_p2p_token(&self, machine_token: &str) -> Result<RobotP2pToken> {
        let domain_id = robot_assignment_hint(machine_token)?;
        let response = self
            .http
            .post(self.endpoint(ROBOT_TOKEN_PATH))
            .bearer_auth(machine_token)
            .json(&RobotP2pTokenRequest { domain_id })
            .send()
            .await
            .map_err(DdsP2pError::Request)?;
        ensure_success(&response)?;
        let response: RobotP2pTokenResponse =
            response.json().await.map_err(DdsP2pError::Request)?;
        if response.p2p_access_token.is_empty() {
            return Err(DdsP2pError::MissingField("p2p_access_token"));
        }
        if response.p2p_access_expires_at <= Utc::now() {
            return Err(DdsP2pError::InvalidExpiration);
        }
        Ok(RobotP2pToken {
            token: response.p2p_access_token,
            expires_at: response.p2p_access_expires_at,
        })
    }

    fn endpoint(&self, path: &str) -> Url {
        let mut endpoint = self.base_url.clone();
        endpoint.set_path(path);
        endpoint.set_query(None);
        endpoint.set_fragment(None);
        endpoint
    }
}

#[derive(Clone)]
pub struct PeerBindingClient {
    dds: DdsP2pClient,
    identity: PeerIdentityProof,
}

impl PeerBindingClient {
    pub fn new(dds: DdsP2pClient, identity: PeerIdentityProof) -> Self {
        Self { dds, identity }
    }

    pub fn peer_id(&self) -> PeerId {
        self.identity.peer_id()
    }

    pub async fn bind(&self, base: &AccessBundle) -> Result<AccessBundle> {
        let challenge = self
            .dds
            .create_challenge(base.token(), &self.identity)
            .await?;
        self.dds
            .verify_challenge(base.token(), &self.identity, challenge)
            .await
    }
}

pub struct ProcessP2p {
    node: Node,
    binding: PeerBindingClient,
    dds: DdsP2pClient,
    authority: DomainAuthority,
    key_refresh_stop: CancellationToken,
    key_refresh_task: Option<JoinHandle<()>>,
}

impl Drop for ProcessP2p {
    fn drop(&mut self) {
        self.key_refresh_stop.cancel();
    }
}

impl ProcessP2p {
    /// Start with an explicitly owned persistent libp2p identity.
    ///
    /// Callers must ensure that the same private key is not used by concurrent
    /// processes. The identity remains process-local after construction and is
    /// never exposed through runner or task APIs.
    pub async fn start_with_identity_and_listen_addresses(
        identity: Identity,
        dds_base_url: Url,
        request_timeout: Duration,
        listen_addresses: impl IntoIterator<Item = Multiaddr>,
    ) -> Result<Self> {
        let dds = DdsP2pClient::new(dds_base_url, request_timeout)?;
        let verifier = dds.initial_verification().await?.verifier.clone();
        let binding = PeerBindingClient::new(dds.clone(), identity.proof());
        let node = Node::start(identity, verifier, listen_addresses).map_err(DdsP2pError::Node)?;
        let authority = node.authority();
        let key_refresh_stop = CancellationToken::new();
        let key_refresh_task = tokio::spawn(drive_verification_key_refresh(
            dds.clone(),
            authority.clone(),
            key_refresh_stop.clone(),
        ));
        Ok(Self {
            node,
            binding,
            dds,
            authority,
            key_refresh_stop,
            key_refresh_task: Some(key_refresh_task),
        })
    }

    pub fn node(&self) -> Node {
        self.node.clone()
    }

    pub fn binding_client(&self) -> PeerBindingClient {
        self.binding.clone()
    }

    pub fn dds_client(&self) -> DdsP2pClient {
        self.dds.clone()
    }

    pub fn authority(&self) -> DomainAuthority {
        self.authority.clone()
    }

    pub fn credential_installer(&self) -> DdsP2pCredentialInstaller {
        DdsP2pCredentialInstaller::new(self.dds.clone(), self.authority.clone())
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.key_refresh_stop.cancel();
        if let Some(task) = self.key_refresh_task.take() {
            if let Err(error) = task.await {
                warn!(error = %error, "DDS P2P verification-key refresh task failed");
            }
        }
        self.node
            .clone()
            .shutdown()
            .await
            .map_err(DdsP2pError::Node)
    }
}

async fn drive_verification_key_refresh(
    dds: DdsP2pClient,
    authority: DomainAuthority,
    stop: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = stop.cancelled() => break,
            _ = tokio::time::sleep(VERIFICATION_KEY_REFRESH_INTERVAL) => {}
        }

        let fetched_keys = tokio::select! {
            _ = stop.cancelled() => break,
            result = tokio::time::timeout(
                VERIFICATION_KEY_REFRESH_TIMEOUT,
                dds.fetch_verification_keys(),
            ) => match result {
                Ok(Ok(keys)) => keys,
                Ok(Err(error)) => {
                    warn!(error = %error, "DDS P2P verification-key refresh failed");
                    continue;
                }
                Err(_) => {
                    warn!("DDS P2P verification-key refresh timed out");
                    continue;
                }
            },
        };
        let install = authority.install_verification_keys(fetched_keys);
        let result = tokio::select! {
            _ = stop.cancelled() => break,
            result = install => result,
        };
        if let Err(error) = result {
            warn!(error = %error, "DDS P2P verification-key update was rejected");
        }
    }
}

/// Installs task credentials only after refreshing their DDS verification set.
///
/// Keeping the DDS client and local authority together prevents a newly issued
/// credential from racing the periodic verification-key refresh task during a
/// signer rotation.
#[derive(Clone)]
pub struct DdsP2pCredentialInstaller {
    dds: DdsP2pClient,
    authority: DomainAuthority,
}

impl DdsP2pCredentialInstaller {
    pub fn new(dds: DdsP2pClient, authority: DomainAuthority) -> Self {
        Self { dds, authority }
    }

    pub async fn install_optional(
        &self,
        token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<Option<P2PAccessClaims>> {
        install_optional_credential(&self.dds, &self.authority, token, expires_at).await
    }

    pub async fn clear(&self) {
        self.authority.clear_credential().await;
    }
}

pub struct RobotP2pTokenProvider {
    relay_auth: RobotP2pRelayAuth,
    domain_id: Uuid,
    stop: CancellationToken,
    task: Option<JoinHandle<()>>,
}

#[derive(Clone)]
struct RobotP2pRelayAuth {
    dds: DdsP2pClient,
    machine_auth: Arc<dyn TokenProvider>,
    authority: DomainAuthority,
    current: Arc<RwLock<Option<RetainedRobotP2pCredential>>>,
    refresh_lock: Arc<Mutex<()>>,
}

struct RetainedRobotP2pCredential {
    bearer: String,
    expires_at: DateTime<Utc>,
}

impl std::fmt::Debug for RetainedRobotP2pCredential {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RetainedRobotP2pCredential")
            .field("bearer", &"[redacted]")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

impl RobotP2pRelayAuth {
    fn new(
        dds: DdsP2pClient,
        machine_auth: Arc<dyn TokenProvider>,
        authority: DomainAuthority,
    ) -> Self {
        Self {
            dds,
            machine_auth,
            authority,
            current: Arc::new(RwLock::new(None)),
            refresh_lock: Arc::new(Mutex::new(())),
        }
    }

    async fn refresh(&self) -> Result<(P2PAccessClaims, Duration)> {
        let _guard = self.refresh_lock.lock().await;
        self.refresh_locked().await
    }

    async fn refresh_locked(&self) -> Result<(P2PAccessClaims, Duration)> {
        let (claims, delay, credential) =
            refresh_robot_token(&self.dds, &self.machine_auth, &self.authority).await?;
        *self.current.write().await = Some(credential);
        Ok((claims, delay))
    }

    async fn refresh_after_unauthorized(&self) {
        let _guard = self.refresh_lock.lock().await;
        self.current.write().await.take();
        if let Err(error) = self.refresh_locked().await {
            warn!(error = %error, "DDS Robot P2P credential refresh after DMS 401 failed");
        }
    }

    async fn clear(&self) {
        self.current.write().await.take();
    }
}

#[async_trait]
impl TokenProvider for RobotP2pRelayAuth {
    async fn bearer(&self) -> std::result::Result<String, TokenProviderError> {
        self.current
            .read()
            .await
            .as_ref()
            .filter(|credential| credential.expires_at > Utc::now())
            .map(|credential| credential.bearer.clone())
            .ok_or_else(|| {
                TokenProviderError::Message("Robot P2P credential is unavailable".to_owned())
            })
    }

    async fn on_unauthorized(&self) {
        self.refresh_after_unauthorized().await;
    }
}

impl RobotP2pTokenProvider {
    pub async fn start(
        dds: DdsP2pClient,
        machine_auth: Arc<dyn TokenProvider>,
        authority: DomainAuthority,
        shutdown: &CancellationToken,
    ) -> Result<Self> {
        let relay_auth = RobotP2pRelayAuth::new(dds, machine_auth, authority);
        let (initial_claims, initial_delay) = relay_auth.refresh().await?;
        let domain_id = initial_claims
            .domain_ids
            .first()
            .and_then(|domain_id| Uuid::parse_str(domain_id).ok())
            .ok_or(DdsP2pError::CredentialDomainMismatch)?;
        let stop = shutdown.child_token();
        let task_relay_auth = relay_auth.clone();
        let task_stop = stop.clone();
        let task = tokio::spawn(async move {
            let mut delay = initial_delay;
            loop {
                tokio::select! {
                    _ = task_stop.cancelled() => break,
                    _ = tokio::time::sleep(delay) => {}
                }
                let refresh = task_relay_auth.refresh();
                let result = tokio::select! {
                    _ = task_stop.cancelled() => break,
                    result = refresh => result,
                };
                match result {
                    Ok((_, next_delay)) => delay = next_delay,
                    Err(error) => {
                        warn!(error = %error, "DDS Robot P2P token refresh failed");
                        delay = REFRESH_RETRY_DELAY;
                    }
                }
            }
        });

        Ok(Self {
            relay_auth,
            domain_id,
            stop,
            task: Some(task),
        })
    }

    pub async fn refresh_now(&self) -> Result<P2PAccessClaims> {
        self.relay_auth.refresh().await.map(|(claims, _)| claims)
    }

    pub fn relay_token_provider(&self) -> Arc<dyn TokenProvider> {
        Arc::new(self.relay_auth.clone())
    }

    pub fn domain_id(&self) -> Uuid {
        self.domain_id
    }

    pub async fn shutdown(mut self) {
        self.stop.cancel();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        self.relay_auth.clear().await;
    }
}

async fn refresh_robot_token(
    dds: &DdsP2pClient,
    machine_auth: &Arc<dyn TokenProvider>,
    authority: &DomainAuthority,
) -> Result<(P2PAccessClaims, Duration, RetainedRobotP2pCredential)> {
    let machine_token = machine_auth
        .bearer()
        .await
        .map_err(DdsP2pError::MachineToken)?;
    let response = dds.robot_p2p_token(&machine_token).await?;
    dds.refresh_verification_keys(authority).await?;
    let credential = SignedP2pCredential::new(response.token.clone()).map_err(DdsP2pError::Node)?;
    let claims = authority
        .install_credential_checked(credential, response.expires_at)
        .await?;
    let remaining = response
        .expires_at
        .signed_duration_since(Utc::now())
        .to_std()
        .map_err(|_| DdsP2pError::InvalidExpiration)?;
    let delay = remaining
        .mul_f64(REFRESH_SAFETY_RATIO)
        .max(Duration::from_secs(1));
    let retained = RetainedRobotP2pCredential {
        bearer: response.token,
        expires_at: response.expires_at,
    };
    Ok((claims, delay, retained))
}

pub async fn install_optional_credential(
    dds: &DdsP2pClient,
    authority: &DomainAuthority,
    token: Option<&str>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<Option<P2PAccessClaims>> {
    match (token, expires_at) {
        (Some(token), Some(expires_at)) => {
            dds.refresh_verification_keys(authority).await?;
            let credential =
                SignedP2pCredential::new(token.to_owned()).map_err(DdsP2pError::Node)?;
            authority
                .install_credential_checked(credential, expires_at)
                .await
                .map(Some)
                .map_err(DdsP2pError::Credential)
        }
        (None, None) => Ok(None),
        _ => Err(DdsP2pError::IncompleteCredential),
    }
}

fn ensure_success(response: &reqwest::Response) -> Result<()> {
    if response.status().is_success() {
        Ok(())
    } else {
        Err(DdsP2pError::UpstreamStatus(response.status()))
    }
}

fn robot_assignment_hint(token: &str) -> Result<Uuid> {
    // This decoded claim is only a request hint. DDS verifies the signed,
    // peer-bound Robot Bearer token and its persisted assignment before it
    // issues a P2P token.
    let mut segments = token.split('.');
    let _header = segments
        .next()
        .filter(|segment| !segment.is_empty())
        .ok_or(DdsP2pError::MalformedMachineToken)?;
    let payload = segments
        .next()
        .filter(|segment| !segment.is_empty())
        .ok_or(DdsP2pError::MalformedMachineToken)?;
    let _signature = segments
        .next()
        .filter(|segment| !segment.is_empty())
        .ok_or(DdsP2pError::MalformedMachineToken)?;
    if segments.next().is_some() {
        return Err(DdsP2pError::MalformedMachineToken);
    }
    let payload = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| DdsP2pError::MalformedMachineToken)?;
    let claims: RobotAssignmentClaims =
        serde_json::from_slice(&payload).map_err(|_| DdsP2pError::MalformedMachineToken)?;
    claims
        .assigned_domain_id
        .ok_or(DdsP2pError::MissingRobotAssignment)
}

#[derive(Serialize)]
struct ChallengeRequest {
    peer_id: String,
    public_key: String,
}

#[derive(Deserialize)]
struct ChallengeResponse {
    challenge_id: String,
    challenge: String,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct VerifyRequest {
    challenge_id: String,
    signature: String,
}

#[derive(Deserialize)]
struct VerifyResponse {
    peer_id: String,
    access_token: String,
    access_expires_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct RobotAssignmentClaims {
    assigned_domain_id: Option<Uuid>,
}

#[derive(Serialize)]
struct RobotP2pTokenRequest {
    domain_id: Uuid,
}

#[derive(Deserialize)]
struct RobotP2pTokenResponse {
    p2p_access_token: String,
    p2p_access_expires_at: DateTime<Utc>,
}

struct RobotP2pToken {
    token: String,
    expires_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DDS_PUBLIC_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEVMaw1idALRBkwGGeONdlTx6jAiqD
8FKYQ5HDiJO0jg7CsFL0yIlK9dTW1wSZmwUX4REXM7LiuD1YWuXoNH1aqA==
-----END PUBLIC KEY-----"#;

    const SECOND_DDS_PUBLIC_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEAxcARQLozLIqu/CFm6ub89EElhHX
O+4eTRPLA8IA+ibNtrfWbavOIYZEtwGneJvRTovHr5OUGFu3n/gXNqGbKw==
-----END PUBLIC KEY-----"#;

    fn response(
        generation: u64,
        current: &str,
        previous: Option<&str>,
    ) -> VerificationKeysResponse {
        let mut keys = vec![VerificationKeyResponse {
            id: verification_key_id(current).unwrap(),
            status: VerificationKeyStatus::Current,
            signing_method: "ES256".into(),
            public_key: current.into(),
        }];
        if let Some(previous) = previous {
            keys.push(VerificationKeyResponse {
                id: verification_key_id(previous).unwrap(),
                status: VerificationKeyStatus::Previous,
                signing_method: "ES256".into(),
                public_key: previous.into(),
            });
        }
        VerificationKeysResponse {
            version: 1,
            generation,
            previous_key_overlap_seconds: DDS_PREVIOUS_KEY_MIN_OVERLAP.as_secs(),
            keys,
        }
    }

    #[tokio::test]
    async fn dds_generations_drive_rotation_restart_and_retirement_shape() {
        let initial = convert_verification_keys(response(41, TEST_DDS_PUBLIC_KEY, None)).unwrap();
        let rotated = convert_verification_keys(response(
            42,
            SECOND_DDS_PUBLIC_KEY,
            Some(TEST_DDS_PUBLIC_KEY),
        ))
        .unwrap();
        let retired = convert_verification_keys(response(43, SECOND_DDS_PUBLIC_KEY, None)).unwrap();

        initial.validate_successor(&rotated).unwrap();
        rotated.validate_successor(&retired).unwrap();

        let verifier = DdsTokenVerifier::from_keys(initial).unwrap();
        let node = Node::start(
            Identity::generate(),
            verifier.clone(),
            std::iter::empty::<Multiaddr>(),
        )
        .unwrap();
        node.authority()
            .install_verification_keys(rotated.clone())
            .await
            .unwrap();
        assert_eq!(verifier.generation(), 42);

        let restarted = DdsTokenVerifier::from_keys(rotated).unwrap();
        assert_eq!(restarted.generation(), 42);
        node.shutdown().await.unwrap();
    }

    #[test]
    fn malformed_key_set_metadata_is_rejected() {
        let mut invalid = response(1, TEST_DDS_PUBLIC_KEY, None);
        invalid.previous_key_overlap_seconds = DDS_PREVIOUS_KEY_MIN_OVERLAP.as_secs() - 1;
        assert!(matches!(
            convert_verification_keys(invalid),
            Err(DdsP2pError::InvalidVerificationKeyResponse(_))
        ));

        let mut invalid = response(1, TEST_DDS_PUBLIC_KEY, None);
        invalid.keys[0].id = "a".repeat(64);
        assert!(matches!(
            convert_verification_keys(invalid),
            Err(DdsP2pError::InvalidVerificationKeyResponse(_))
        ));

        let mut invalid = response(1, TEST_DDS_PUBLIC_KEY, None);
        invalid.keys[0].signing_method = "RS256".into();
        assert!(matches!(
            convert_verification_keys(invalid),
            Err(DdsP2pError::InvalidVerificationKeyResponse(_))
        ));
    }

    #[test]
    fn retained_robot_p2p_credential_debug_is_redacted() {
        let credential = RetainedRobotP2pCredential {
            bearer: "sensitive-robot-p2p-bearer".to_owned(),
            expires_at: Utc::now(),
        };

        let rendered = format!("{credential:?}");
        assert!(rendered.contains("[redacted]"));
        assert!(!rendered.contains("sensitive-robot-p2p-bearer"));
    }
}

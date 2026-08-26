use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub use auki_p2p::DomainAuthority;
use auki_p2p::{
    DdsTokenVerifier, DdsVerificationKeys, Identity, Multiaddr, Node, P2PAccessClaims, PeerId,
    SignedP2pCredential, DDS_PREVIOUS_KEY_MIN_OVERLAP, DDS_VERIFICATION_KEY_MAX_BYTES,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::{sync::OnceCell, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use url::Url;
use uuid::Uuid;

use crate::auth::{AccessBundle, TokenProvider, TokenProviderError};

const CHALLENGE_PATH: &str = "/internal/v1/auth/p2p/challenge";
const VERIFY_PATH: &str = "/internal/v1/auth/p2p/verify";
const ROBOT_TOKEN_PATH: &str = "/internal/v1/auth/robot/p2p-token";
const PUBLIC_KEY_PATH: &str = "/service/public-key.pem";
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
    #[error("DDS P2P verification-key generation is exhausted")]
    VerificationKeyGenerationExhausted,
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
    current_pem: Vec<u8>,
}

#[derive(Clone)]
struct VerificationKeyRefreshState {
    generation: u64,
    current_pem: Vec<u8>,
    previous_pem: Option<Vec<u8>>,
    rotated_at: Option<Instant>,
}

struct VerificationKeyRefreshProposal {
    preferred: VerificationKeyRefreshState,
    rotation_fallback: Option<VerificationKeyRefreshState>,
}

impl VerificationKeyRefreshState {
    fn new(current_pem: Vec<u8>) -> Self {
        Self {
            generation: 0,
            current_pem,
            previous_pem: None,
            rotated_at: None,
        }
    }

    fn propose(
        &self,
        fetched_pem: Vec<u8>,
        now: Instant,
    ) -> Result<VerificationKeyRefreshProposal> {
        let raw_encoding_changed = fetched_pem != self.current_pem;
        let retire_previous = self.previous_pem.is_some()
            && self.rotated_at.is_some_and(|rotated_at| {
                now.duration_since(rotated_at) >= DDS_PREVIOUS_KEY_MIN_OVERLAP
            });
        let changes_generation = raw_encoding_changed || retire_previous;
        let preferred = Self {
            generation: if changes_generation {
                self.next_generation()?
            } else {
                self.generation
            },
            current_pem: fetched_pem.clone(),
            previous_pem: if retire_previous {
                None
            } else {
                self.previous_pem.clone()
            },
            rotated_at: if retire_previous {
                None
            } else {
                self.rotated_at
            },
        };
        let rotation_fallback = raw_encoding_changed.then(|| Self {
            generation: preferred.generation,
            current_pem: fetched_pem,
            previous_pem: Some(self.current_pem.clone()),
            rotated_at: Some(now),
        });

        Ok(VerificationKeyRefreshProposal {
            preferred,
            rotation_fallback,
        })
    }

    fn next_generation(&self) -> Result<u64> {
        self.generation
            .checked_add(1)
            .ok_or(DdsP2pError::VerificationKeyGenerationExhausted)
    }

    fn verification_keys(&self) -> DdsVerificationKeys {
        DdsVerificationKeys::new(
            self.generation,
            self.current_pem.clone(),
            self.previous_pem.clone(),
        )
    }
}

impl DdsP2pClient {
    pub fn new(base_url: Url, timeout: Duration) -> Result<Self> {
        let http = Client::builder()
            .use_rustls_tls()
            .no_proxy()
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
                let current_pem = self.fetch_verification_key().await?;
                let verifier = DdsTokenVerifier::from_es256_pem(&current_pem)
                    .map_err(DdsP2pError::InvalidPublicKey)?;
                Ok(InitialVerification {
                    verifier,
                    current_pem,
                })
            })
            .await
    }

    async fn fetch_verification_key(&self) -> Result<Vec<u8>> {
        let response = self
            .http
            .get(self.endpoint(PUBLIC_KEY_PATH))
            .send()
            .await
            .map_err(DdsP2pError::Request)?;
        ensure_success(&response)?;
        let mut pem = Vec::new();
        let mut body = response.bytes_stream();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(DdsP2pError::Request)?;
            if pem.len().saturating_add(chunk.len()) > DDS_VERIFICATION_KEY_MAX_BYTES {
                return Err(DdsP2pError::VerificationKeyResponseTooLarge);
            }
            pem.extend_from_slice(&chunk);
        }
        if pem.is_empty() {
            return Err(DdsP2pError::MissingField("DDS public key"));
        }
        Ok(pem)
    }

    async fn create_challenge(
        &self,
        machine_token: &str,
        authority: &DomainAuthority,
    ) -> Result<ChallengeResponse> {
        let request = ChallengeRequest {
            peer_id: authority.peer_id().to_string(),
            public_key: URL_SAFE_NO_PAD.encode(authority.peer_public_key_protobuf()),
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
        authority: &DomainAuthority,
        challenge: ChallengeResponse,
    ) -> Result<AccessBundle> {
        let challenge_bytes = URL_SAFE_NO_PAD
            .decode(challenge.challenge)
            .map_err(DdsP2pError::InvalidChallengeEncoding)?;
        let signature = authority
            .sign_peer_challenge(&challenge_bytes)
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
        if response.peer_id != authority.peer_id().to_string() {
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
    authority: DomainAuthority,
}

impl PeerBindingClient {
    pub fn new(dds: DdsP2pClient, authority: DomainAuthority) -> Self {
        Self { dds, authority }
    }

    pub fn peer_id(&self) -> PeerId {
        self.authority.peer_id()
    }

    pub async fn bind(&self, base: &AccessBundle) -> Result<AccessBundle> {
        let challenge = self
            .dds
            .create_challenge(base.token(), &self.authority)
            .await?;
        self.dds
            .verify_challenge(base.token(), &self.authority, challenge)
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
    pub async fn start(dds_base_url: Url, request_timeout: Duration) -> Result<Self> {
        Self::start_with_listen_addresses(dds_base_url, request_timeout, []).await
    }

    pub async fn start_with_listen_addresses(
        dds_base_url: Url,
        request_timeout: Duration,
        listen_addresses: impl IntoIterator<Item = Multiaddr>,
    ) -> Result<Self> {
        Self::start_with_identity_and_listen_addresses(
            Identity::generate(),
            dds_base_url,
            request_timeout,
            listen_addresses,
        )
        .await
    }

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
        let (verifier, initial_current_pem) = {
            let initial = dds.initial_verification().await?;
            (initial.verifier.clone(), initial.current_pem.clone())
        };
        let node = Node::start(identity, verifier, listen_addresses).map_err(DdsP2pError::Node)?;
        let authority = node.authority();
        let binding = PeerBindingClient::new(dds.clone(), authority.clone());
        let key_refresh_stop = CancellationToken::new();
        let key_refresh_task = tokio::spawn(drive_verification_key_refresh(
            dds.clone(),
            authority.clone(),
            key_refresh_stop.clone(),
            VerificationKeyRefreshState::new(initial_current_pem),
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
    mut state: VerificationKeyRefreshState,
) {
    loop {
        tokio::select! {
            _ = stop.cancelled() => break,
            _ = tokio::time::sleep(VERIFICATION_KEY_REFRESH_INTERVAL) => {}
        }

        let fetched_pem = tokio::select! {
            _ = stop.cancelled() => break,
            result = tokio::time::timeout(
                VERIFICATION_KEY_REFRESH_TIMEOUT,
                dds.fetch_verification_key(),
            ) => match result {
                Ok(Ok(pem)) => pem,
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
        let install =
            install_verification_key_refresh(&authority, &state, fetched_pem, Instant::now());
        let result = tokio::select! {
            _ = stop.cancelled() => break,
            result = install => result,
        };
        match result {
            Ok(installed) => state = installed,
            Err(error) => {
                warn!(error = %error, "DDS P2P verification-key update was rejected");
            }
        }
    }
}

async fn install_verification_key_refresh(
    authority: &DomainAuthority,
    state: &VerificationKeyRefreshState,
    fetched_pem: Vec<u8>,
    now: Instant,
) -> Result<VerificationKeyRefreshState> {
    let proposal = state.propose(fetched_pem, now)?;
    match authority
        .install_verification_keys(proposal.preferred.verification_keys())
        .await
    {
        Ok(()) => Ok(proposal.preferred),
        Err(auki_p2p::Error::VerificationKeyRotationMissingPrevious) => {
            let fallback = proposal
                .rotation_fallback
                .ok_or(auki_p2p::Error::VerificationKeyRotationMissingPrevious)
                .map_err(DdsP2pError::Node)?;
            authority
                .install_verification_keys(fallback.verification_keys())
                .await
                .map_err(DdsP2pError::Node)?;
            Ok(fallback)
        }
        Err(error) => Err(DdsP2pError::Node(error)),
    }
}

pub struct RobotP2pTokenProvider {
    dds: DdsP2pClient,
    machine_auth: Arc<dyn TokenProvider>,
    authority: DomainAuthority,
    domain_id: Uuid,
    stop: CancellationToken,
    task: Option<JoinHandle<()>>,
}

impl RobotP2pTokenProvider {
    pub async fn start(
        dds: DdsP2pClient,
        machine_auth: Arc<dyn TokenProvider>,
        authority: DomainAuthority,
        shutdown: &CancellationToken,
    ) -> Result<Self> {
        let (initial_claims, initial_delay) =
            refresh_robot_token(&dds, &machine_auth, &authority).await?;
        let domain_id = initial_claims
            .domain_ids
            .first()
            .and_then(|domain_id| Uuid::parse_str(domain_id).ok())
            .ok_or(DdsP2pError::CredentialDomainMismatch)?;
        let stop = shutdown.child_token();
        let task_dds = dds.clone();
        let task_auth = Arc::clone(&machine_auth);
        let task_authority = authority.clone();
        let task_stop = stop.clone();
        let task = tokio::spawn(async move {
            let mut delay = initial_delay;
            loop {
                tokio::select! {
                    _ = task_stop.cancelled() => break,
                    _ = tokio::time::sleep(delay) => {}
                }
                let refresh = refresh_robot_token(&task_dds, &task_auth, &task_authority);
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
            dds,
            machine_auth,
            authority,
            domain_id,
            stop,
            task: Some(task),
        })
    }

    pub async fn refresh_now(&self) -> Result<P2PAccessClaims> {
        refresh_robot_token(&self.dds, &self.machine_auth, &self.authority)
            .await
            .map(|(claims, _)| claims)
    }

    pub fn domain_id(&self) -> Uuid {
        self.domain_id
    }

    pub async fn shutdown(mut self) {
        self.stop.cancel();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

async fn refresh_robot_token(
    dds: &DdsP2pClient,
    machine_auth: &Arc<dyn TokenProvider>,
    authority: &DomainAuthority,
) -> Result<(P2PAccessClaims, Duration)> {
    let machine_token = machine_auth
        .bearer()
        .await
        .map_err(DdsP2pError::MachineToken)?;
    let response = dds.robot_p2p_token(&machine_token).await?;
    let credential = SignedP2pCredential::new(response.token).map_err(DdsP2pError::Node)?;
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
    Ok((claims, delay))
}

pub async fn install_optional_credential(
    authority: &DomainAuthority,
    token: Option<&str>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<Option<P2PAccessClaims>> {
    match (token, expires_at) {
        (Some(token), Some(expires_at)) => {
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

    const TEST_DDS_PUBLIC_KEY: &[u8] = br#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEVMaw1idALRBkwGGeONdlTx6jAiqD
8FKYQ5HDiJO0jg7CsFL0yIlK9dTW1wSZmwUX4REXM7LiuD1YWuXoNH1aqA==
-----END PUBLIC KEY-----"#;

    const SECOND_DDS_PUBLIC_KEY: &[u8] = br#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEAxcARQLozLIqu/CFm6ub89EElhHX
O+4eTRPLA8IA+ibNtrfWbavOIYZEtwGneJvRTovHr5OUGFu3n/gXNqGbKw==
-----END PUBLIC KEY-----"#;

    #[test]
    fn verification_key_refresh_tracks_rotation_overlap_and_retirement() {
        let started_at = Instant::now();
        let initial = VerificationKeyRefreshState::new(b"key-a".to_vec());

        let refreshed = initial
            .propose(b"key-a".to_vec(), started_at)
            .expect("same key refreshes generation zero")
            .preferred;
        assert_eq!(refreshed.generation, 0);
        assert_eq!(refreshed.current_pem, b"key-a");
        assert!(refreshed.previous_pem.is_none());

        let rotation = refreshed
            .propose(b"key-b".to_vec(), started_at)
            .expect("new key creates a rotation generation");
        assert_eq!(rotation.preferred.generation, 1);
        assert_eq!(rotation.preferred.current_pem, b"key-b");
        assert!(rotation.preferred.previous_pem.is_none());
        let rotated = rotation
            .rotation_fallback
            .expect("a byte change has a true-rotation fallback");
        assert_eq!(rotated.generation, 1);
        assert_eq!(rotated.current_pem, b"key-b");
        assert_eq!(rotated.previous_pem.as_deref(), Some(b"key-a".as_slice()));
        assert_eq!(rotated.rotated_at, Some(started_at));

        let protected = rotated
            .propose(
                b"key-b".to_vec(),
                started_at + DDS_PREVIOUS_KEY_MIN_OVERLAP - Duration::from_nanos(1),
            )
            .expect("same key keeps the protected previous key")
            .preferred;
        assert_eq!(protected.generation, 1);
        assert_eq!(protected.previous_pem.as_deref(), Some(b"key-a".as_slice()));

        let retired = protected
            .propose(b"key-b".to_vec(), started_at + DDS_PREVIOUS_KEY_MIN_OVERLAP)
            .expect("previous key retires after the full overlap")
            .preferred;
        assert_eq!(retired.generation, 2);
        assert_eq!(retired.current_pem, b"key-b");
        assert!(retired.previous_pem.is_none());
        assert!(retired.rotated_at.is_none());
    }

    #[test]
    fn verification_key_refresh_never_wraps_generation() {
        let exhausted = VerificationKeyRefreshState {
            generation: u64::MAX,
            current_pem: b"key-a".to_vec(),
            previous_pem: None,
            rotated_at: None,
        };

        assert!(matches!(
            exhausted.propose(b"key-b".to_vec(), Instant::now()),
            Err(DdsP2pError::VerificationKeyGenerationExhausted)
        ));
    }

    #[tokio::test]
    async fn equivalent_pem_reencoding_refreshes_without_creating_a_duplicate_previous_key() {
        let verifier = DdsTokenVerifier::from_es256_pem(TEST_DDS_PUBLIC_KEY).unwrap();
        let node = Node::start(
            Identity::generate(),
            verifier,
            std::iter::empty::<Multiaddr>(),
        )
        .unwrap();
        let authority = node.authority();
        let state = VerificationKeyRefreshState::new(TEST_DDS_PUBLIC_KEY.to_vec());
        let mut equivalent_pem = TEST_DDS_PUBLIC_KEY.to_vec();
        equivalent_pem.push(b'\n');

        let installed = install_verification_key_refresh(
            &authority,
            &state,
            equivalent_pem.clone(),
            Instant::now(),
        )
        .await
        .expect("the SDK accepts a higher-generation canonical-key refresh");

        assert_eq!(installed.generation, 1);
        assert_eq!(installed.current_pem, equivalent_pem);
        assert!(installed.previous_pem.is_none());
        node.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn distinct_key_uses_the_rotation_fallback_and_preserves_the_old_key() {
        let verifier = DdsTokenVerifier::from_es256_pem(TEST_DDS_PUBLIC_KEY).unwrap();
        let node = Node::start(
            Identity::generate(),
            verifier,
            std::iter::empty::<Multiaddr>(),
        )
        .unwrap();
        let authority = node.authority();
        let state = VerificationKeyRefreshState::new(TEST_DDS_PUBLIC_KEY.to_vec());
        let rotated_at = Instant::now();

        let installed = install_verification_key_refresh(
            &authority,
            &state,
            SECOND_DDS_PUBLIC_KEY.to_vec(),
            rotated_at,
        )
        .await
        .expect("the SDK lineage rejection selects the true-rotation fallback");

        assert_eq!(installed.generation, 1);
        assert_eq!(installed.current_pem, SECOND_DDS_PUBLIC_KEY);
        assert_eq!(installed.previous_pem.as_deref(), Some(TEST_DDS_PUBLIC_KEY));
        assert_eq!(installed.rotated_at, Some(rotated_at));
        node.shutdown().await.unwrap();
    }
}

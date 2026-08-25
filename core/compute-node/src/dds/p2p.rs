use std::{sync::Arc, time::Duration};

pub use auki_p2p::P2pCredentialStore;
use auki_p2p::{DdsTokenVerifier, Identity, Multiaddr, Node, P2PAccessClaims, PeerId};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
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
    verifier: Arc<OnceCell<DdsTokenVerifier>>,
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
            verifier: Arc::new(OnceCell::new()),
        })
    }

    pub async fn token_verifier(&self) -> Result<DdsTokenVerifier> {
        self.verifier
            .get_or_try_init(|| async {
                let response = self
                    .http
                    .get(self.endpoint(PUBLIC_KEY_PATH))
                    .send()
                    .await
                    .map_err(DdsP2pError::Request)?;
                ensure_success(&response)?;
                let pem = response.bytes().await.map_err(DdsP2pError::Request)?;
                if pem.is_empty() {
                    return Err(DdsP2pError::MissingField("DDS public key"));
                }
                DdsTokenVerifier::from_es256_pem(&pem).map_err(DdsP2pError::InvalidPublicKey)
            })
            .await
            .cloned()
    }

    async fn create_challenge(
        &self,
        machine_token: &str,
        identity: &Identity,
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
        identity: &Identity,
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
    identity: Identity,
}

impl PeerBindingClient {
    pub fn new(dds: DdsP2pClient, identity: Identity) -> Self {
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
    credentials: P2pCredentialStore,
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
        let verifier = dds.token_verifier().await?;
        let node =
            Node::start(identity.clone(), verifier, listen_addresses).map_err(DdsP2pError::Node)?;
        let binding = PeerBindingClient::new(dds.clone(), identity);
        let credentials = P2pCredentialStore::new(node.clone());
        Ok(Self {
            node,
            binding,
            dds,
            credentials,
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

    pub fn credentials(&self) -> P2pCredentialStore {
        self.credentials.clone()
    }

    pub async fn shutdown(self) -> Result<()> {
        self.node.shutdown().await.map_err(DdsP2pError::Node)
    }
}

pub struct RobotP2pTokenProvider {
    dds: DdsP2pClient,
    machine_auth: Arc<dyn TokenProvider>,
    credentials: P2pCredentialStore,
    domain_id: Uuid,
    stop: CancellationToken,
    task: Option<JoinHandle<()>>,
}

impl RobotP2pTokenProvider {
    pub async fn start(
        dds: DdsP2pClient,
        machine_auth: Arc<dyn TokenProvider>,
        credentials: P2pCredentialStore,
        shutdown: &CancellationToken,
    ) -> Result<Self> {
        let (initial_claims, initial_delay) =
            refresh_robot_token(&dds, &machine_auth, &credentials).await?;
        let domain_id = initial_claims
            .domain_ids
            .first()
            .and_then(|domain_id| Uuid::parse_str(domain_id).ok())
            .ok_or(DdsP2pError::CredentialDomainMismatch)?;
        let stop = shutdown.child_token();
        let task_dds = dds.clone();
        let task_auth = Arc::clone(&machine_auth);
        let task_credentials = credentials.clone();
        let task_stop = stop.clone();
        let task = tokio::spawn(async move {
            let mut delay = initial_delay;
            loop {
                tokio::select! {
                    _ = task_stop.cancelled() => break,
                    _ = tokio::time::sleep(delay) => {}
                }
                let refresh = refresh_robot_token(&task_dds, &task_auth, &task_credentials);
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
            credentials,
            domain_id,
            stop,
            task: Some(task),
        })
    }

    pub async fn refresh_now(&self) -> Result<P2PAccessClaims> {
        refresh_robot_token(&self.dds, &self.machine_auth, &self.credentials)
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
    credentials: &P2pCredentialStore,
) -> Result<(P2PAccessClaims, Duration)> {
    let machine_token = machine_auth
        .bearer()
        .await
        .map_err(DdsP2pError::MachineToken)?;
    let response = dds.robot_p2p_token(&machine_token).await?;
    let claims = credentials
        .install(response.token, response.expires_at)
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

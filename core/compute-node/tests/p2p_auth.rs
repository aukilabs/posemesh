use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use async_trait::async_trait;
use auki_p2p::{
    DdsTokenVerifier, Identity, Node, P2PAccessClaims, PeerRole, DDS_VERIFICATION_KEY_MAX_BYTES,
    P2P_TOKEN_AUDIENCE, P2P_TOKEN_ISSUER, P2P_TOKEN_SCOPE, P2P_TOKEN_TTL, P2P_TOKEN_TYPE,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use httpmock::{prelude::*, Mock};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use p256::pkcs8::{DecodePublicKey, EncodePublicKey};
use posemesh_compute_node::{
    auth::{
        token_manager::{TokenManagerConfig, TokenProviderError, TokenProviderResult},
        AccessBundle, RobotMachineAuth, TokenProvider,
    },
    dds::p2p::{
        DdsP2pClient, DdsP2pCredentialInstaller, DdsP2pError, DomainAuthority, PeerBindingClient,
        RobotP2pTokenProvider,
    },
    dms::types::HeartbeatResponse,
    engine::{apply_heartbeat_token_update, apply_p2p_credential_update},
    storage::TokenRef,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const CHALLENGE_PATH: &str = "/internal/v1/auth/p2p/challenge";
const VERIFY_PATH: &str = "/internal/v1/auth/p2p/verify";
const ROBOT_REGISTER_PATH: &str = "/internal/v1/robots/register";
const ROBOT_VERIFY_PATH: &str = "/internal/v1/auth/robot/verify";
const ROBOT_P2P_TOKEN_PATH: &str = "/internal/v1/auth/robot/p2p-token";
const VERIFICATION_KEYS_PATH: &str = "/service/p2p-verification-keys";

const TEST_DDS_PRIVATE_KEY: &[u8] = br#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQggm4twpf4y/yNNw/k
fqecEEl4zBTwZdRDFUFp/fSxV8qhRANCAARUxrDWJ0AtEGTAYZ4412VPHqMCKoPw
UphDkcOIk7SODsKwUvTIiUr11NbXBJmbBRfhERczsuK4PVha5eg0fVqo
-----END PRIVATE KEY-----"#;

const TEST_DDS_PUBLIC_KEY: &[u8] = br#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEVMaw1idALRBkwGGeONdlTx6jAiqD
8FKYQ5HDiJO0jg7CsFL0yIlK9dTW1wSZmwUX4REXM7LiuD1YWuXoNH1aqA==
-----END PUBLIC KEY-----"#;

const SECOND_DDS_PRIVATE_KEY: &[u8] = br#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgwRbuxaM6rEI3vYEl
vRmIEsc1QtC3uPMWvXo1xXt+CcOhRANCAAQDFwBFAujMsiq78IWbq5vz0QSWEdc7
7h5NE8sDwgD6Js22t9Ztq84hhkS3Aad4m9FOi8evk5QYW7ef+Bc2oZsr
-----END PRIVATE KEY-----"#;

const SECOND_DDS_PUBLIC_KEY: &[u8] = br#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEAxcARQLozLIqu/CFm6ub89EElhHX
O+4eTRPLA8IA+ibNtrfWbavOIYZEtwGneJvRTovHr5OUGFu3n/gXNqGbKw==
-----END PUBLIC KEY-----"#;

fn client(server: &MockServer) -> DdsP2pClient {
    DdsP2pClient::new(server.base_url().parse().unwrap(), Duration::from_secs(2)).unwrap()
}

fn verification_key_id(public_key: &[u8]) -> String {
    let public_key =
        p256::PublicKey::from_public_key_pem(std::str::from_utf8(public_key).unwrap()).unwrap();
    let der = public_key.to_public_key_der().unwrap();
    hex::encode(Sha256::digest(der.as_bytes()))
}

fn verification_keys_body(
    generation: u64,
    current: &[u8],
    previous: Option<&[u8]>,
) -> serde_json::Value {
    let mut keys = vec![json!({
        "id": verification_key_id(current),
        "status": "current",
        "signing_method": "ES256",
        "public_key": std::str::from_utf8(current).unwrap(),
    })];
    if let Some(previous) = previous {
        keys.push(json!({
            "id": verification_key_id(previous),
            "status": "previous",
            "signing_method": "ES256",
            "public_key": std::str::from_utf8(previous).unwrap(),
        }));
    }
    json!({
        "version": 1,
        "generation": generation,
        "previous_key_overlap_seconds": 1860,
        "keys": keys,
    })
}

fn verification_keys_mock<'a>(
    server: &'a MockServer,
    generation: u64,
    current: &[u8],
    previous: Option<&[u8]>,
) -> Mock<'a> {
    let body = verification_keys_body(generation, current, previous);
    server.mock(move |when, then| {
        when.method(GET)
            .path(VERIFICATION_KEYS_PATH)
            .header("accept", "application/json")
            .header("cache-control", "no-cache");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(body.clone());
    })
}

fn node_with_identity(identity: Identity) -> Node {
    Node::start(
        identity,
        DdsTokenVerifier::from_es256_pem(TEST_DDS_PUBLIC_KEY).unwrap(),
        std::iter::empty::<auki_p2p::Multiaddr>(),
    )
    .unwrap()
}

fn binding_mocks<'a>(
    server: &'a MockServer,
    authority: &DomainAuthority,
    base_token: &str,
    bound_token: &str,
    challenge_id: &str,
    challenge_bytes: &[u8],
) -> (Mock<'a>, Mock<'a>) {
    let peer_id = authority.peer_id().to_string();
    let public_key = URL_SAFE_NO_PAD.encode(authority.peer_public_key_protobuf());
    let challenge = URL_SAFE_NO_PAD.encode(challenge_bytes);
    let signature = URL_SAFE_NO_PAD.encode(authority.sign_peer_challenge(challenge_bytes).unwrap());
    let expires_at = Utc::now() + chrono::Duration::minutes(10);
    let challenge_mock = server.mock(|when, then| {
        when.method(POST)
            .path(CHALLENGE_PATH)
            .header("authorization", format!("Bearer {base_token}"))
            .json_body(json!({
                "peer_id": peer_id,
                "public_key": public_key,
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "challenge_id": challenge_id,
                "challenge": challenge,
                "expires_at": expires_at,
            }));
    });
    let verify_mock = server.mock(|when, then| {
        when.method(POST)
            .path(VERIFY_PATH)
            .header("authorization", format!("Bearer {base_token}"))
            .json_body(json!({
                "challenge_id": challenge_id,
                "signature": signature,
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "peer_id": authority.peer_id().to_string(),
                "access_token": bound_token,
                "access_expires_at": expires_at,
            }));
    });
    (challenge_mock, verify_mock)
}

#[tokio::test]
async fn compute_challenge_uses_the_exact_process_identity_without_robot_exchange() {
    let server = MockServer::start();
    let identity = Identity::generate();
    let node = node_with_identity(identity.clone());
    let authority = node.authority();
    let binding = PeerBindingClient::new(client(&server), authority.clone());
    let (challenge, verify) = binding_mocks(
        &server,
        &authority,
        "compute-base-token",
        "compute-peer-bound-token",
        "compute-challenge",
        b"compute challenge bytes",
    );
    let robot_exchange = server.mock(|when, then| {
        when.method(POST).path(ROBOT_P2P_TOKEN_PATH);
        then.status(500);
    });

    let bound = binding
        .bind(&AccessBundle::new(
            "compute-base-token",
            Utc::now() + chrono::Duration::minutes(10),
        ))
        .await
        .expect("Compute binding succeeds");

    assert_eq!(bound.token(), "compute-peer-bound-token");
    assert_eq!(binding.peer_id(), identity.peer_id());
    challenge.assert_hits(1);
    verify.assert_hits(1);
    robot_exchange.assert_hits(0);
    node.shutdown().await.unwrap();
}

#[tokio::test]
async fn robot_base_token_refresh_requires_a_new_peer_challenge() {
    let server = MockServer::start();
    let identity = Identity::generate();
    let node = node_with_identity(identity.clone());
    let authority = node.authority();
    let binding = PeerBindingClient::new(client(&server), authority.clone());
    let robot_id = Uuid::new_v4();
    let credentials = "opaque-robot-credentials";
    let capabilities = vec!["/robot/demo/v0".to_string()];
    let expires_at = Utc::now() + chrono::Duration::hours(1);

    let register = server.mock(|when, then| {
        when.method(POST).path(ROBOT_REGISTER_PATH);
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "robot_id": robot_id,
                "access_token": "robot-base-a",
                "access_expires_at": expires_at,
            }));
    });
    let refresh = server.mock(|when, then| {
        when.method(POST).path(ROBOT_VERIFY_PATH);
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "robot_id": robot_id,
                "access_token": "robot-base-b",
                "access_expires_at": expires_at,
            }));
    });
    let (challenge_a, verify_a) = binding_mocks(
        &server,
        &authority,
        "robot-base-a",
        "robot-bound-a",
        "robot-challenge-a",
        b"first robot challenge",
    );
    let (challenge_b, verify_b) = binding_mocks(
        &server,
        &authority,
        "robot-base-b",
        "robot-bound-b",
        "robot-challenge-b",
        b"second robot challenge",
    );

    let auth = RobotMachineAuth::new_peer_bound(
        server.base_url().parse().unwrap(),
        credentials,
        "1.0.0",
        capabilities,
        Duration::from_secs(2),
        TokenManagerConfig {
            safety_ratio: 0.75,
            max_retries: 0,
            jitter: Duration::ZERO,
        },
        binding,
    )
    .unwrap();
    let handle = auth.start().await.expect("Robot peer binding starts");
    assert_eq!(handle.bearer().await.unwrap(), "robot-bound-a");

    handle.on_unauthorized().await;
    assert_eq!(handle.bearer().await.unwrap(), "robot-bound-b");
    handle.shutdown().await;

    register.assert_hits(1);
    refresh.assert_hits(1);
    challenge_a.assert_hits(1);
    verify_a.assert_hits(1);
    challenge_b.assert_hits(1);
    verify_b.assert_hits(1);
    node.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_or_replayed_peer_proof_fails_closed() {
    let server = MockServer::start();
    let identity = Identity::generate();
    let node = node_with_identity(identity);
    let authority = node.authority();
    let binding = PeerBindingClient::new(client(&server), authority.clone());
    let base_token = "proof-must-not-appear-in-error";
    let challenge_bytes = b"one-time challenge";
    let challenge = server.mock(|when, then| {
        when.method(POST)
            .path(CHALLENGE_PATH)
            .header("authorization", format!("Bearer {base_token}"));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "challenge_id": "already-consumed",
                "challenge": URL_SAFE_NO_PAD.encode(challenge_bytes),
                "expires_at": Utc::now() + chrono::Duration::minutes(1),
            }));
    });
    let verify = server.mock(|when, then| {
        when.method(POST)
            .path(VERIFY_PATH)
            .header("authorization", format!("Bearer {base_token}"))
            .json_body(json!({
                "challenge_id": "already-consumed",
                "signature": URL_SAFE_NO_PAD.encode(
                    authority.sign_peer_challenge(challenge_bytes).unwrap()
                ),
            }));
        then.status(401);
    });

    let error = binding
        .bind(&AccessBundle::new(
            base_token,
            Utc::now() + chrono::Duration::minutes(10),
        ))
        .await
        .expect_err("consumed proof must fail");

    assert!(matches!(
        error,
        DdsP2pError::UpstreamStatus(reqwest::StatusCode::UNAUTHORIZED)
    ));
    assert!(!error.to_string().contains(base_token));
    challenge.assert_hits(1);
    verify.assert_hits(1);
    node.shutdown().await.unwrap();
}

#[tokio::test]
async fn dds_verification_key_set_is_fetched_once_and_cached() {
    let server = MockServer::start();
    let key_set = verification_keys_mock(&server, 7, TEST_DDS_PUBLIC_KEY, None);
    let client = client(&server);

    client.token_verifier().await.expect("first key fetch");
    client.token_verifier().await.expect("cached key fetch");

    key_set.assert_hits(1);
}

#[tokio::test]
async fn dds_verification_key_response_is_bounded_before_parsing() {
    let server = MockServer::start();
    let key_set = server.mock(|when, then| {
        when.method(GET).path(VERIFICATION_KEYS_PATH);
        then.status(200)
            .header("content-type", "application/json")
            .body("x".repeat(DDS_VERIFICATION_KEY_MAX_BYTES + 1));
    });

    let error = match client(&server).token_verifier().await {
        Err(error) => error,
        Ok(_) => panic!("oversized key response must fail closed"),
    };

    assert!(matches!(
        error,
        DdsP2pError::VerificationKeyResponseTooLarge
    ));
    key_set.assert_hits(1);
}

#[tokio::test]
async fn dds_verification_key_response_rejects_unknown_fields() {
    let server = MockServer::start();
    let mut body = verification_keys_body(1, TEST_DDS_PUBLIC_KEY, None);
    body["unexpected"] = json!(true);
    let key_set = server.mock(|when, then| {
        when.method(GET).path(VERIFICATION_KEYS_PATH);
        then.status(200)
            .header("content-type", "application/json")
            .json_body(body.clone());
    });

    let error = match client(&server).token_verifier().await {
        Err(error) => error,
        Ok(_) => panic!("unknown key-set fields must fail closed"),
    };
    assert!(matches!(
        error,
        DdsP2pError::InvalidVerificationKeyResponse(_)
    ));
    key_set.assert_hits(1);
}

#[tokio::test]
async fn restart_during_rotation_trusts_the_published_current_and_previous_keys() {
    let server = MockServer::start();
    let key_set = verification_keys_mock(
        &server,
        12,
        SECOND_DDS_PUBLIC_KEY,
        Some(TEST_DDS_PUBLIC_KEY),
    );
    let identity = Identity::generate();
    let domain_id = Uuid::new_v4();
    let issued_at = Utc::now().timestamp() as u64;
    let (old_token, _) = signed_p2p_token_at_with_key(
        &identity,
        domain_id,
        PeerRole::Compute,
        Uuid::new_v4(),
        issued_at,
        TEST_DDS_PRIVATE_KEY,
    );
    let (new_token, _) = signed_p2p_token_at_with_key(
        &identity,
        domain_id,
        PeerRole::Compute,
        Uuid::new_v4(),
        issued_at,
        SECOND_DDS_PRIVATE_KEY,
    );

    let first = client(&server).token_verifier().await.unwrap();
    assert_eq!(first.generation(), 12);
    first.verify(&old_token).unwrap();
    first.verify(&new_token).unwrap();

    let restarted = client(&server).token_verifier().await.unwrap();
    assert_eq!(restarted.generation(), 12);
    restarted.verify(&old_token).unwrap();
    restarted.verify(&new_token).unwrap();
    key_set.assert_hits(2);
}

#[derive(Clone)]
struct StaticMachineToken(String);

#[async_trait]
impl TokenProvider for StaticMachineToken {
    async fn bearer(&self) -> TokenProviderResult<String> {
        Ok(self.0.clone())
    }

    async fn on_unauthorized(&self) {}
}

struct SequencedMachineTokens {
    tokens: Vec<String>,
    next: AtomicUsize,
}

impl SequencedMachineTokens {
    fn new(tokens: Vec<String>) -> Self {
        Self {
            tokens,
            next: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl TokenProvider for SequencedMachineTokens {
    async fn bearer(&self) -> TokenProviderResult<String> {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        self.tokens
            .get(index)
            .cloned()
            .ok_or_else(|| TokenProviderError::Message("no test machine token".to_owned()))
    }

    async fn on_unauthorized(&self) {}
}

#[tokio::test]
async fn heartbeat_replaces_p2p_credentials_without_mixing_domain_http_tokens() {
    let server = MockServer::start();
    let key_set = verification_keys_mock(&server, 1, TEST_DDS_PUBLIC_KEY, None);
    let identity = Identity::generate();
    let verifier = DdsTokenVerifier::from_es256_pem(TEST_DDS_PUBLIC_KEY).unwrap();
    let node = Node::start(
        identity.clone(),
        verifier,
        std::iter::empty::<auki_p2p::Multiaddr>(),
    )
    .unwrap();
    let credentials = node.authority();
    let installer = DdsP2pCredentialInstaller::new(client(&server), credentials.clone());
    let domain_id = Uuid::new_v4();
    let first_subject = Uuid::new_v4();
    let first_issued_at = Utc::now().timestamp() as u64;
    let (first_token, first_expiry) = signed_p2p_token_at(
        &identity,
        domain_id,
        PeerRole::Compute,
        first_subject,
        first_issued_at,
    );
    apply_p2p_credential_update(Some(&installer), Some(&first_token), Some(first_expiry))
        .await
        .unwrap();
    assert_eq!(
        credentials.current_claims().await.unwrap().sub,
        first_subject.to_string()
    );

    let domain_token = TokenRef::new("domain-http-a".into());
    let second_subject = Uuid::new_v4();
    let (second_token, second_expiry) = signed_p2p_token_at(
        &identity,
        domain_id,
        PeerRole::Compute,
        second_subject,
        first_issued_at + 1,
    );
    let p2p_heartbeat = HeartbeatResponse {
        p2p_access_token: Some(second_token),
        p2p_access_token_expires_at: Some(second_expiry),
        ..HeartbeatResponse::default()
    };
    apply_heartbeat_token_update(&domain_token, &p2p_heartbeat);
    apply_p2p_credential_update(
        Some(&installer),
        p2p_heartbeat.p2p_access_token.as_deref(),
        p2p_heartbeat.p2p_access_token_expires_at,
    )
    .await
    .unwrap();
    assert_eq!(domain_token.get(), "domain-http-a");
    assert_eq!(
        credentials.current_claims().await.unwrap().sub,
        second_subject.to_string()
    );

    let domain_heartbeat = HeartbeatResponse {
        access_token: Some("domain-http-b".into()),
        ..HeartbeatResponse::default()
    };
    apply_heartbeat_token_update(&domain_token, &domain_heartbeat);
    apply_p2p_credential_update(
        Some(&installer),
        domain_heartbeat.p2p_access_token.as_deref(),
        domain_heartbeat.p2p_access_token_expires_at,
    )
    .await
    .unwrap();
    assert_eq!(domain_token.get(), "domain-http-b");
    assert_eq!(
        credentials.current_claims().await.unwrap().sub,
        second_subject.to_string()
    );

    credentials.clear_credential().await;
    key_set.assert_hits(2);
    node.shutdown().await.unwrap();
}

#[tokio::test]
async fn compute_credential_refreshes_rotated_keys_before_verification() {
    let server = MockServer::start();
    let key_set =
        verification_keys_mock(&server, 2, SECOND_DDS_PUBLIC_KEY, Some(TEST_DDS_PUBLIC_KEY));
    let identity = Identity::generate();
    let verifier = DdsTokenVerifier::from_es256_pem(TEST_DDS_PUBLIC_KEY).unwrap();
    let node = Node::start(
        identity.clone(),
        verifier.clone(),
        std::iter::empty::<auki_p2p::Multiaddr>(),
    )
    .unwrap();
    let authority = node.authority();
    let installer = DdsP2pCredentialInstaller::new(client(&server), authority.clone());
    let domain_id = Uuid::new_v4();
    let (token, expires_at) = signed_p2p_token_at_with_key(
        &identity,
        domain_id,
        PeerRole::Compute,
        Uuid::new_v4(),
        Utc::now().timestamp() as u64,
        SECOND_DDS_PRIVATE_KEY,
    );

    apply_p2p_credential_update(Some(&installer), Some(&token), Some(expires_at))
        .await
        .unwrap();

    assert_eq!(verifier.generation(), 2);
    assert_eq!(
        authority.current_claims().await.unwrap().domain_ids,
        vec![domain_id.to_string()]
    );
    key_set.assert_hits(1);
    node.shutdown().await.unwrap();
}

#[tokio::test]
async fn robot_direct_token_refresh_hot_swaps_and_shuts_down_cleanly() {
    let server = MockServer::start();
    let key_set = verification_keys_mock(&server, 1, TEST_DDS_PUBLIC_KEY, None);
    let identity = Identity::generate();
    let domain_id = Uuid::new_v4();
    let machine_token = robot_machine_token(Some(domain_id));
    let (p2p_token, expires_at) = signed_robot_p2p_token(&identity, domain_id);
    let exchange = server.mock(|when, then| {
        when.method(POST)
            .path(ROBOT_P2P_TOKEN_PATH)
            .header("authorization", format!("Bearer {machine_token}"))
            .json_body(json!({"domain_id": domain_id}));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "p2p_access_token": p2p_token,
                "p2p_access_expires_at": expires_at,
            }));
    });
    let verifier = DdsTokenVerifier::from_es256_pem(TEST_DDS_PUBLIC_KEY).unwrap();
    let node = Node::start(
        identity.clone(),
        verifier,
        std::iter::empty::<auki_p2p::Multiaddr>(),
    )
    .unwrap();
    let machine_auth: Arc<dyn TokenProvider> = Arc::new(StaticMachineToken(machine_token));
    let shutdown = CancellationToken::new();
    let credentials = node.authority();
    let provider =
        RobotP2pTokenProvider::start(client(&server), machine_auth, credentials, &shutdown)
            .await
            .expect("initial Robot P2P refresh");

    let refreshed = provider.refresh_now().await.expect("direct refresh");
    assert_eq!(
        refreshed.peer_type.as_deref(),
        Some(PeerRole::Robot.as_str())
    );
    assert_eq!(refreshed.peer_id, identity.peer_id().to_string());
    assert_eq!(refreshed.domain_ids, vec![domain_id.to_string()]);
    exchange.assert_hits(2);
    key_set.assert_hits(2);

    tokio::time::timeout(Duration::from_secs(1), provider.shutdown())
        .await
        .expect("provider shutdown must not hang");
    node.shutdown().await.unwrap();
}

#[tokio::test]
async fn robot_relay_auth_refreshes_the_p2p_credential_after_unauthorized() {
    let server = MockServer::start();
    let key_set = verification_keys_mock(&server, 1, TEST_DDS_PUBLIC_KEY, None);
    let identity = Identity::generate();
    let domain_id = Uuid::new_v4();
    let machine_token_a = format!("{}-a", robot_machine_token(Some(domain_id)));
    let machine_token_b = format!("{}-b", robot_machine_token(Some(domain_id)));
    let issued_at = Utc::now().timestamp() as u64;
    let (p2p_token_a, expires_at_a) = signed_p2p_token_at(
        &identity,
        domain_id,
        PeerRole::Robot,
        Uuid::new_v4(),
        issued_at,
    );
    let (p2p_token_b, expires_at_b) = signed_p2p_token_at(
        &identity,
        domain_id,
        PeerRole::Robot,
        Uuid::new_v4(),
        issued_at + 1,
    );
    let exchange_a = server.mock(|when, then| {
        when.method(POST)
            .path(ROBOT_P2P_TOKEN_PATH)
            .header("authorization", format!("Bearer {machine_token_a}"));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "p2p_access_token": p2p_token_a,
                "p2p_access_expires_at": expires_at_a,
            }));
    });
    let exchange_b = server.mock(|when, then| {
        when.method(POST)
            .path(ROBOT_P2P_TOKEN_PATH)
            .header("authorization", format!("Bearer {machine_token_b}"));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "p2p_access_token": p2p_token_b,
                "p2p_access_expires_at": expires_at_b,
            }));
    });
    let node = Node::start(
        identity,
        DdsTokenVerifier::from_es256_pem(TEST_DDS_PUBLIC_KEY).unwrap(),
        std::iter::empty::<auki_p2p::Multiaddr>(),
    )
    .unwrap();
    let machine_auth: Arc<dyn TokenProvider> = Arc::new(SequencedMachineTokens::new(vec![
        machine_token_a,
        machine_token_b,
    ]));
    let shutdown = CancellationToken::new();
    let provider =
        RobotP2pTokenProvider::start(client(&server), machine_auth, node.authority(), &shutdown)
            .await
            .unwrap();
    let relay_auth = provider.relay_token_provider();

    assert_eq!(relay_auth.bearer().await.unwrap(), p2p_token_a);
    relay_auth.on_unauthorized().await;
    assert_eq!(relay_auth.bearer().await.unwrap(), p2p_token_b);
    exchange_a.assert_hits(1);
    exchange_b.assert_hits(1);
    key_set.assert_hits(2);

    provider.shutdown().await;
    assert!(relay_auth.bearer().await.is_err());
    node.shutdown().await.unwrap();
}

#[tokio::test]
async fn robot_direct_token_refresh_rejects_missing_assignment_without_http() {
    let server = MockServer::start();
    let exchange = server.mock(|when, then| {
        when.method(POST).path(ROBOT_P2P_TOKEN_PATH);
        then.status(200);
    });
    let identity = Identity::generate();
    let verifier = DdsTokenVerifier::from_es256_pem(TEST_DDS_PUBLIC_KEY).unwrap();
    let node = Node::start(
        identity,
        verifier,
        std::iter::empty::<auki_p2p::Multiaddr>(),
    )
    .unwrap();
    let machine_auth: Arc<dyn TokenProvider> =
        Arc::new(StaticMachineToken(robot_machine_token(None)));
    let shutdown = CancellationToken::new();
    let credentials = node.authority();

    let error =
        match RobotP2pTokenProvider::start(client(&server), machine_auth, credentials, &shutdown)
            .await
        {
            Ok(provider) => {
                provider.shutdown().await;
                panic!("unassigned Robot must fail closed");
            }
            Err(error) => error,
        };

    assert!(matches!(error, DdsP2pError::MissingRobotAssignment));
    exchange.assert_hits(0);
    node.shutdown().await.unwrap();
}

fn robot_machine_token(assigned_domain_id: Option<Uuid>) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"ES256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&json!({
            "sub": Uuid::new_v4(),
            "assigned_domain_id": assigned_domain_id,
        }))
        .unwrap(),
    );
    format!("{header}.{payload}.signature")
}

fn signed_robot_p2p_token(identity: &Identity, domain_id: Uuid) -> (String, DateTime<Utc>) {
    signed_p2p_token(identity, domain_id, PeerRole::Robot, Uuid::new_v4())
}

fn signed_p2p_token(
    identity: &Identity,
    domain_id: Uuid,
    role: PeerRole,
    subject: Uuid,
) -> (String, DateTime<Utc>) {
    let issued_at = Utc::now().timestamp() as u64;
    signed_p2p_token_at(identity, domain_id, role, subject, issued_at)
}

fn signed_p2p_token_at(
    identity: &Identity,
    domain_id: Uuid,
    role: PeerRole,
    subject: Uuid,
    issued_at: u64,
) -> (String, DateTime<Utc>) {
    signed_p2p_token_at_with_key(
        identity,
        domain_id,
        role,
        subject,
        issued_at,
        TEST_DDS_PRIVATE_KEY,
    )
}

fn signed_p2p_token_at_with_key(
    identity: &Identity,
    domain_id: Uuid,
    role: PeerRole,
    subject: Uuid,
    issued_at: u64,
    signing_key: &[u8],
) -> (String, DateTime<Utc>) {
    let expires_at_unix = issued_at + P2P_TOKEN_TTL.as_secs();
    let expires_at = DateTime::from_timestamp(expires_at_unix as i64, 0).unwrap();
    let claims = P2PAccessClaims {
        token_type: P2P_TOKEN_TYPE.into(),
        iss: P2P_TOKEN_ISSUER.into(),
        aud: vec![P2P_TOKEN_AUDIENCE.into()],
        sub: subject.to_string(),
        organization_id: None,
        peer_type: Some(role.to_string()),
        peer_id: identity.peer_id().to_string(),
        domain_ids: vec![domain_id.to_string()],
        scopes: vec![P2P_TOKEN_SCOPE.into()],
        application: None,
        iat: issued_at,
        nbf: None,
        exp: expires_at_unix,
    };
    let token = encode(
        &Header::new(Algorithm::ES256),
        &claims,
        &EncodingKey::from_ec_pem(signing_key).unwrap(),
    )
    .unwrap();
    (token, expires_at)
}

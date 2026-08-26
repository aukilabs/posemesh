use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use auki_p2p::{
    DdsTokenVerifier, Identity, Node, P2PAccessClaims, PeerRole, DDS_VERIFICATION_KEY_MAX_BYTES,
    P2P_TOKEN_AUDIENCE, P2P_TOKEN_ISSUER, P2P_TOKEN_SCOPE, P2P_TOKEN_TTL, P2P_TOKEN_TYPE,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use httpmock::{prelude::*, Mock};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use posemesh_compute_node::{
    auth::{token_manager::TokenManagerConfig, AccessBundle, RobotMachineAuth, TokenProvider},
    dds::p2p::{
        DdsP2pClient, DdsP2pError, DomainAuthority, PeerBindingClient, RobotP2pTokenProvider,
    },
    dms::types::HeartbeatResponse,
    engine::{apply_heartbeat_token_update, apply_p2p_credential_update},
    storage::TokenRef,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const CHALLENGE_PATH: &str = "/internal/v1/auth/p2p/challenge";
const VERIFY_PATH: &str = "/internal/v1/auth/p2p/verify";
const ROBOT_REGISTER_PATH: &str = "/internal/v1/robots/register";
const ROBOT_VERIFY_PATH: &str = "/internal/v1/auth/robot/verify";
const ROBOT_P2P_TOKEN_PATH: &str = "/internal/v1/auth/robot/p2p-token";
const PUBLIC_KEY_PATH: &str = "/service/public-key.pem";

const TEST_DDS_PRIVATE_KEY: &[u8] = br#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQggm4twpf4y/yNNw/k
fqecEEl4zBTwZdRDFUFp/fSxV8qhRANCAARUxrDWJ0AtEGTAYZ4412VPHqMCKoPw
UphDkcOIk7SODsKwUvTIiUr11NbXBJmbBRfhERczsuK4PVha5eg0fVqo
-----END PRIVATE KEY-----"#;

const TEST_DDS_PUBLIC_KEY: &[u8] = br#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEVMaw1idALRBkwGGeONdlTx6jAiqD
8FKYQ5HDiJO0jg7CsFL0yIlK9dTW1wSZmwUX4REXM7LiuD1YWuXoNH1aqA==
-----END PUBLIC KEY-----"#;

fn client(server: &MockServer) -> DdsP2pClient {
    DdsP2pClient::new(server.base_url().parse().unwrap(), Duration::from_secs(2)).unwrap()
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
async fn dds_public_key_is_fetched_once_and_cached() {
    let server = MockServer::start();
    let public_key = server.mock(|when, then| {
        when.method(GET).path(PUBLIC_KEY_PATH);
        then.status(200).body(TEST_DDS_PUBLIC_KEY);
    });
    let client = client(&server);

    client.token_verifier().await.expect("first key fetch");
    client.token_verifier().await.expect("cached key fetch");

    public_key.assert_hits(1);
}

#[tokio::test]
async fn dds_public_key_response_is_bounded_before_parsing() {
    let server = MockServer::start();
    let public_key = server.mock(|when, then| {
        when.method(GET).path(PUBLIC_KEY_PATH);
        then.status(200)
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
    public_key.assert_hits(1);
}

#[derive(Clone)]
struct StaticMachineToken(String);

#[async_trait]
impl TokenProvider for StaticMachineToken {
    async fn bearer(
        &self,
    ) -> posemesh_compute_node::auth::token_manager::TokenProviderResult<String> {
        Ok(self.0.clone())
    }

    async fn on_unauthorized(&self) {}
}

#[tokio::test]
async fn heartbeat_replaces_p2p_credentials_without_mixing_domain_http_tokens() {
    let identity = Identity::generate();
    let verifier = DdsTokenVerifier::from_es256_pem(TEST_DDS_PUBLIC_KEY).unwrap();
    let node = Node::start(
        identity.clone(),
        verifier,
        std::iter::empty::<auki_p2p::Multiaddr>(),
    )
    .unwrap();
    let credentials = node.authority();
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
    apply_p2p_credential_update(Some(&credentials), Some(&first_token), Some(first_expiry))
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
        Some(&credentials),
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
        Some(&credentials),
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
    node.shutdown().await.unwrap();
}

#[tokio::test]
async fn robot_direct_token_refresh_hot_swaps_and_shuts_down_cleanly() {
    let server = MockServer::start();
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

    tokio::time::timeout(Duration::from_secs(1), provider.shutdown())
        .await
        .expect("provider shutdown must not hang");
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
    let expires_at_unix = issued_at + P2P_TOKEN_TTL.as_secs();
    let expires_at = DateTime::from_timestamp(expires_at_unix as i64, 0).unwrap();
    let claims = P2PAccessClaims {
        token_type: P2P_TOKEN_TYPE.into(),
        iss: P2P_TOKEN_ISSUER.into(),
        aud: vec![P2P_TOKEN_AUDIENCE.into()],
        sub: subject.to_string(),
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
        &EncodingKey::from_ec_pem(TEST_DDS_PRIVATE_KEY).unwrap(),
    )
    .unwrap();
    (token, expires_at)
}

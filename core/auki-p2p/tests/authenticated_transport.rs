use std::time::{Duration, SystemTime, UNIX_EPOCH};

use auki_p2p::{
    ApplicationProtocol, DdsTokenVerifier, Error, ExactRoute, Identity, Node, P2PAccessClaims,
    PeerRole, ProtocolSpec, SessionRequirements, DOMAIN_SERVER_MAX_DOMAINS, P2P_TOKEN_AUDIENCE,
    P2P_TOKEN_ISSUER, P2P_TOKEN_SCOPE, P2P_TOKEN_TTL, P2P_TOKEN_TYPE,
};
use futures::io::{AsyncReadExt, AsyncWriteExt};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use libp2p::{identity::PublicKey, Multiaddr};
use serde::Serialize;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const TEST_DDS_PRIVATE_KEY: &[u8] = br#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQggm4twpf4y/yNNw/k
fqecEEl4zBTwZdRDFUFp/fSxV8qhRANCAARUxrDWJ0AtEGTAYZ4412VPHqMCKoPw
UphDkcOIk7SODsKwUvTIiUr11NbXBJmbBRfhERczsuK4PVha5eg0fVqo
-----END PRIVATE KEY-----"#;

const TEST_DDS_PUBLIC_KEY: &[u8] = br#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEVMaw1idALRBkwGGeONdlTx6jAiqD
8FKYQ5HDiJO0jg7CsFL0yIlK9dTW1wSZmwUX4REXM7LiuD1YWuXoNH1aqA==
-----END PUBLIC KEY-----"#;

const TEST_PROTOCOL: &str = "/auki-p2p/test/0.1.0";
type ClaimsMutation = Box<dyn Fn(&mut P2PAccessClaims)>;

#[test]
fn identity_uses_libp2p_ed25519_proofs() {
    let identity = Identity::generate();
    let public_key = PublicKey::try_decode_protobuf(&identity.public_key_protobuf()).unwrap();
    assert_eq!(public_key.to_peer_id(), identity.peer_id());

    let challenge = b"synthetic DDS challenge bytes\0with exact payload";
    let signature = identity.sign_challenge(challenge).unwrap();
    assert!(public_key.verify(challenge, &signature));
    assert!(!public_key.verify(b"different challenge", &signature));
}

#[test]
fn verifier_enforces_the_exact_dds_claim_profile() {
    let verifier = verifier();
    let identity = Identity::generate();
    let domain_id = Uuid::new_v4().to_string();
    let now = unix_time();

    for role in [PeerRole::Robot, PeerRole::Compute, PeerRole::DomainServer] {
        let single_domain = claims(&identity, role, vec![domain_id.clone()], now);
        let verified = verifier.verify(&sign(&single_domain)).unwrap();
        assert_eq!(verified.peer_type, role);

        let multi_domain = claims(
            &identity,
            role,
            vec![domain_id.clone(), Uuid::new_v4().to_string()],
            now,
        );
        assert_eq!(
            verifier
                .verify(&sign(&multi_domain))
                .unwrap()
                .domain_ids
                .len(),
            2
        );
    }

    let mut cases: Vec<(&str, ClaimsMutation)> = vec![
        (
            "wrong type",
            Box::new(|claims| claims.token_type = "other".into()),
        ),
        ("wrong issuer", Box::new(|claims| claims.iss = "api".into())),
        ("missing audience", Box::new(|claims| claims.aud.clear())),
        (
            "extra audience",
            Box::new(|claims| claims.aud.push("other".into())),
        ),
        (
            "invalid subject",
            Box::new(|claims| claims.sub = "not-a-uuid".into()),
        ),
        (
            "invalid peer id",
            Box::new(|claims| claims.peer_id = "not-a-peer".into()),
        ),
        (
            "missing domain",
            Box::new(|claims| claims.domain_ids.clear()),
        ),
        (
            "invalid domain",
            Box::new(|claims| claims.domain_ids = vec!["not-a-uuid".into()]),
        ),
        (
            "duplicate domain",
            Box::new(|claims| {
                claims.peer_type = PeerRole::DomainServer;
                claims.domain_ids.push(claims.domain_ids[0].clone());
            }),
        ),
        ("missing scope", Box::new(|claims| claims.scopes.clear())),
        (
            "unknown scope",
            Box::new(|claims| claims.scopes = vec!["domain-data:w".into()]),
        ),
        (
            "short ttl",
            Box::new(|claims| claims.exp = claims.iat + P2P_TOKEN_TTL.as_secs() - 1),
        ),
        (
            "long ttl",
            Box::new(|claims| claims.exp = claims.iat + P2P_TOKEN_TTL.as_secs() + 1),
        ),
    ];

    for (name, mutate) in cases.drain(..) {
        let mut invalid = claims(&identity, PeerRole::Robot, vec![domain_id.clone()], now);
        mutate(&mut invalid);
        assert!(verifier.verify(&sign(&invalid)).is_err(), "accepted {name}");
    }

    let too_many_domains = (0..=DOMAIN_SERVER_MAX_DOMAINS)
        .map(|_| Uuid::new_v4().to_string())
        .collect();
    let too_many_claims = claims(&identity, PeerRole::DomainServer, too_many_domains, now);
    assert!(verifier.verify(&sign(&too_many_claims)).is_err());

    let maximum_domains = (0..DOMAIN_SERVER_MAX_DOMAINS)
        .map(|_| Uuid::new_v4().to_string())
        .collect();
    let maximum_claims = claims(&identity, PeerRole::DomainServer, maximum_domains, now);
    assert!(verifier.verify(&sign(&maximum_claims)).is_ok());

    let valid_claims = claims(
        &identity,
        PeerRole::Robot,
        vec![Uuid::new_v4().to_string()],
        now,
    );
    let mut unknown_role = serde_json::to_value(&valid_claims).unwrap();
    unknown_role["peer_type"] = serde_json::json!("user");
    assert!(verifier.verify(&sign(&unknown_role)).is_err());

    let mut missing_issued_at = serde_json::to_value(&valid_claims).unwrap();
    missing_issued_at.as_object_mut().unwrap().remove("iat");
    assert!(verifier.verify(&sign(&missing_issued_at)).is_err());

    let mut missing_expiration = serde_json::to_value(&valid_claims).unwrap();
    missing_expiration.as_object_mut().unwrap().remove("exp");
    assert!(verifier.verify(&sign(&missing_expiration)).is_err());

    let expired = claims(
        &identity,
        PeerRole::Robot,
        vec![domain_id],
        now - P2P_TOKEN_TTL.as_secs() - 1,
    );
    assert!(verifier.verify(&sign(&expired)).is_err());

    let mut bad_signature = sign(&valid_claims).into_bytes();
    let last = bad_signature.last_mut().unwrap();
    *last = if *last == b'A' { b'B' } else { b'A' };
    assert!(verifier
        .verify(&String::from_utf8(bad_signature).unwrap())
        .is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn peers_exchange_bytes_only_after_mutual_authentication() {
    let domain_id = Uuid::new_v4().to_string();
    let robot = listening_node();
    let compute = listening_node();
    install_current_token(&robot, PeerRole::Robot, vec![domain_id.clone()]).await;
    install_current_token(&compute, PeerRole::Compute, vec![domain_id.clone()]).await;

    let protocol = ApplicationProtocol::new(TEST_PROTOCOL).unwrap();
    let mut incoming = robot
        .accept(
            protocol.clone(),
            SessionRequirements::new(&domain_id, PeerRole::Compute).unwrap(),
        )
        .unwrap();
    let robot_peer_id = robot.peer_id();
    let robot_address = listen_address(&robot).await;

    let server = tokio::spawn(async move {
        let mut stream = incoming.accept().await.unwrap().unwrap();
        assert_eq!(stream.remote_peer().role, PeerRole::Compute);
        let mut request = [0; 4];
        stream.read_exact(&mut request).await.unwrap();
        assert_eq!(&request, b"ping");
        stream.write_all(b"pong").await.unwrap();
        stream.flush().await.unwrap();
    });

    let mut stream = compute
        .open(
            robot_peer_id,
            vec![robot_address],
            protocol,
            SessionRequirements::new(&domain_id, PeerRole::Robot)
                .unwrap()
                .with_expected_remote_peer_id(robot_peer_id),
        )
        .await
        .unwrap();
    assert_eq!(stream.remote_peer().peer_id, robot_peer_id);
    stream.write_all(b"ping").await.unwrap();
    stream.flush().await.unwrap();
    let mut response = [0; 4];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(&response, b"pong");
    server.await.unwrap();

    robot.shutdown().await.unwrap();
    compute.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn one_runtime_supervises_multiple_independent_authenticated_protocols() {
    let domain_id = Uuid::new_v4().to_string();
    let robot = listening_node();
    let compute = listening_node();
    install_current_token(&robot, PeerRole::Robot, vec![domain_id.clone()]).await;
    install_current_token(&compute, PeerRole::Compute, vec![domain_id.clone()]).await;
    let robot_peer_id = robot.peer_id();
    let robot_address = listen_address(&robot).await;
    let shutdown = CancellationToken::new();

    let mut servers = Vec::new();
    for (name, response) in [
        ("/auki-p2p/runtime-alpha/1", b"alpha".as_slice()),
        ("/auki-p2p/runtime-beta/1", b"beta".as_slice()),
    ] {
        let spec = ProtocolSpec::new(
            ApplicationProtocol::new(name).unwrap(),
            SessionRequirements::new(&domain_id, PeerRole::Compute).unwrap(),
        );
        servers.push(
            robot
                .serve(spec, &shutdown, move |mut stream| async move {
                    let mut request = [0_u8; 1];
                    stream.read_exact(&mut request).await.unwrap();
                    assert_eq!(request, [1]);
                    stream.write_all(response).await.unwrap();
                    stream.flush().await.unwrap();
                })
                .unwrap(),
        );
    }

    for (name, expected) in [
        ("/auki-p2p/runtime-alpha/1", b"alpha".as_slice()),
        ("/auki-p2p/runtime-beta/1", b"beta".as_slice()),
    ] {
        let mut stream = compute
            .open_exact_route(
                robot_peer_id,
                ExactRoute::Direct(robot_address.clone()),
                ApplicationProtocol::new(name).unwrap(),
                SessionRequirements::new(&domain_id, PeerRole::Robot)
                    .unwrap()
                    .with_expected_remote_peer_id(robot_peer_id),
            )
            .await
            .unwrap();
        stream.write_all(&[1]).await.unwrap();
        stream.flush().await.unwrap();
        let mut response = vec![0_u8; expected.len()];
        stream.read_exact(&mut response).await.unwrap();
        assert_eq!(response, expected);
        stream.close().await.unwrap();
    }

    for server in servers {
        server.shutdown().await.unwrap();
    }
    robot.shutdown().await.unwrap();
    compute.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn wrong_domain_is_rejected_before_application_bytes() {
    let robot_domain = Uuid::new_v4().to_string();
    let compute_domain = Uuid::new_v4().to_string();
    let (robot_error, compute_error) = rejected_session(
        PeerRole::Compute,
        robot_domain.clone(),
        compute_domain,
        robot_domain,
    )
    .await;

    assert!(matches!(robot_error, Error::RemoteDomainMismatch(_)));
    assert!(matches!(compute_error, Error::RemoteRejected));
}

#[tokio::test(flavor = "multi_thread")]
async fn disallowed_role_is_rejected_before_application_bytes() {
    let domain_id = Uuid::new_v4().to_string();
    let (robot_error, compute_error) = rejected_session(
        PeerRole::DomainServer,
        domain_id.clone(),
        domain_id.clone(),
        domain_id,
    )
    .await;

    assert!(matches!(robot_error, Error::RemoteRoleMismatch { .. }));
    assert!(matches!(compute_error, Error::RemoteRejected));
}

#[tokio::test(flavor = "multi_thread")]
async fn expired_installed_token_is_rechecked_at_session_time() {
    let domain_id = Uuid::new_v4().to_string();
    let robot = listening_node();
    let compute = listening_node();
    install_current_token(&robot, PeerRole::Robot, vec![domain_id.clone()]).await;

    let nearly_expired = claims(
        compute.identity(),
        PeerRole::Compute,
        vec![domain_id.clone()],
        unix_time() - P2P_TOKEN_TTL.as_secs() + 1,
    );
    compute.install_token(sign(&nearly_expired)).await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;

    let protocol = ApplicationProtocol::new(TEST_PROTOCOL).unwrap();
    let mut incoming = robot
        .accept(
            protocol.clone(),
            SessionRequirements::new(&domain_id, PeerRole::Compute).unwrap(),
        )
        .unwrap();
    let robot_peer_id = robot.peer_id();
    let server = tokio::spawn(async move { incoming.accept().await.unwrap().unwrap_err() });
    let client_error = compute
        .open(
            robot_peer_id,
            vec![listen_address(&robot).await],
            protocol,
            SessionRequirements::new(&domain_id, PeerRole::Robot).unwrap(),
        )
        .await
        .unwrap_err();

    assert!(matches!(client_error, Error::TokenVerification(_)));
    assert!(matches!(server.await.unwrap(), Error::TokenVerification(_)));
    robot.shutdown().await.unwrap();
    compute.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn copied_token_cannot_be_installed_for_another_noise_identity() {
    let domain_id = Uuid::new_v4().to_string();
    let original_identity = Identity::generate();
    let copied_claims = claims(
        &original_identity,
        PeerRole::Compute,
        vec![domain_id],
        unix_time(),
    );
    let other_node = listening_node();

    let error = other_node
        .install_token(sign(&copied_claims))
        .await
        .unwrap_err();
    assert!(matches!(error, Error::PeerIdMismatch { .. }));
    other_node.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_stream_after_disconnect_performs_a_fresh_handshake() {
    let domain_id = Uuid::new_v4().to_string();
    let wrong_domain_id = Uuid::new_v4().to_string();
    let robot = listening_node();
    let compute = listening_node();
    install_current_token(&robot, PeerRole::Robot, vec![domain_id.clone()]).await;
    install_current_token(&compute, PeerRole::Compute, vec![domain_id.clone()]).await;

    let protocol = ApplicationProtocol::new(TEST_PROTOCOL).unwrap();
    let mut incoming = robot
        .accept(
            protocol.clone(),
            SessionRequirements::new(&domain_id, PeerRole::Compute).unwrap(),
        )
        .unwrap();
    let robot_peer_id = robot.peer_id();
    let robot_address = listen_address(&robot).await;

    let server = tokio::spawn(async move {
        let mut first = incoming.accept().await.unwrap().unwrap();
        let mut request = [0; 1];
        first.read_exact(&mut request).await.unwrap();
        first.write_all(&request).await.unwrap();
        first.flush().await.unwrap();
        drop(first);

        incoming.accept().await.unwrap().unwrap_err()
    });

    let requirements = SessionRequirements::new(&domain_id, PeerRole::Robot)
        .unwrap()
        .with_expected_remote_peer_id(robot_peer_id);
    let mut first = compute
        .open(
            robot_peer_id,
            vec![robot_address.clone()],
            protocol.clone(),
            requirements.clone(),
        )
        .await
        .unwrap();
    first.write_all(b"1").await.unwrap();
    first.flush().await.unwrap();
    let mut response = [0; 1];
    first.read_exact(&mut response).await.unwrap();
    assert_eq!(&response, b"1");
    drop(first);

    install_current_token(&robot, PeerRole::Robot, vec![wrong_domain_id]).await;
    compute.disconnect(robot_peer_id).await.unwrap();

    let error = compute
        .open(robot_peer_id, vec![robot_address], protocol, requirements)
        .await
        .unwrap_err();
    assert!(
        matches!(error, Error::RemoteDomainMismatch(_)),
        "unexpected reconnect error: {error:?}"
    );
    assert!(matches!(server.await.unwrap(), Error::RemoteRejected));

    robot.shutdown().await.unwrap();
    compute.shutdown().await.unwrap();
}

async fn rejected_session(
    accepted_compute_role: PeerRole,
    robot_domain: String,
    compute_domain: String,
    required_domain: String,
) -> (Error, Error) {
    let robot = listening_node();
    let compute = listening_node();
    install_current_token(&robot, PeerRole::Robot, vec![robot_domain]).await;
    install_current_token(&compute, PeerRole::Compute, vec![compute_domain]).await;

    let protocol = ApplicationProtocol::new(TEST_PROTOCOL).unwrap();
    let mut incoming = robot
        .accept(
            protocol.clone(),
            SessionRequirements::new(&required_domain, accepted_compute_role).unwrap(),
        )
        .unwrap();
    let robot_peer_id = robot.peer_id();
    let robot_address = listen_address(&robot).await;
    let server = tokio::spawn(async move { incoming.accept().await.unwrap().unwrap_err() });
    let client_error = compute
        .open(
            robot_peer_id,
            vec![robot_address],
            protocol,
            SessionRequirements::new(&required_domain, PeerRole::Robot).unwrap(),
        )
        .await
        .unwrap_err();
    let server_error = server.await.unwrap();

    robot.shutdown().await.unwrap();
    compute.shutdown().await.unwrap();
    (server_error, client_error)
}

fn listening_node() -> Node {
    Node::start(
        Identity::generate(),
        verifier(),
        ["/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap()],
    )
    .unwrap()
}

async fn listen_address(node: &Node) -> Multiaddr {
    tokio::time::timeout(Duration::from_secs(5), node.first_listen_address())
        .await
        .expect("listener did not start")
        .unwrap()
}

async fn install_current_token(node: &Node, role: PeerRole, domain_ids: Vec<String>) {
    let claims = claims(node.identity(), role, domain_ids, unix_time());
    node.install_token(sign(&claims)).await.unwrap();
}

fn claims(
    identity: &Identity,
    role: PeerRole,
    domain_ids: Vec<String>,
    issued_at: u64,
) -> P2PAccessClaims {
    P2PAccessClaims {
        token_type: P2P_TOKEN_TYPE.into(),
        iss: P2P_TOKEN_ISSUER.into(),
        aud: vec![P2P_TOKEN_AUDIENCE.into()],
        sub: Uuid::new_v4().to_string(),
        peer_type: role,
        peer_id: identity.peer_id().to_string(),
        domain_ids,
        scopes: vec![P2P_TOKEN_SCOPE.into()],
        iat: issued_at,
        exp: issued_at + P2P_TOKEN_TTL.as_secs(),
    }
}

fn sign(claims: &impl Serialize) -> String {
    encode(
        &Header::new(Algorithm::ES256),
        claims,
        &EncodingKey::from_ec_pem(TEST_DDS_PRIVATE_KEY).unwrap(),
    )
    .unwrap()
}

fn verifier() -> DdsTokenVerifier {
    DdsTokenVerifier::from_es256_pem(TEST_DDS_PUBLIC_KEY).unwrap()
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

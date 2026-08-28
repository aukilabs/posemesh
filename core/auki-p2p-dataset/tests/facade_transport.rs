use std::{
    net::TcpListener,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use auki_p2p::{
    P2PAccessClaims, P2P_TOKEN_AUDIENCE, P2P_TOKEN_ISSUER, P2P_TOKEN_SCOPE, P2P_TOKEN_TTL,
    P2P_TOKEN_TYPE,
};
use auki_p2p_dataset::{
    DatasetRoutePolicy, P2pDatasetAdapter, P2pDatasetError, P2pDatasetRegistration,
};
use auki_sdk::{
    AukiPeer, AukiPeerConfig, DdsVerificationKeys, ExternalAuthorityUpdate, Identity, Multiaddr,
    SignedP2pCredential,
};
use chrono::{TimeZone, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use tempfile::TempDir;
use tokio::fs;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn direct_only_facade_peers_transfer_one_domain_dataset() {
    tokio::time::timeout(Duration::from_secs(10), run_direct_only_transfer())
        .await
        .expect("facade dataset transfer exceeded its test deadline");
}

async fn run_direct_only_transfer() {
    let temp = TempDir::new().unwrap();
    let domain_id = Uuid::new_v4();
    let robot_identity = Identity::generate();
    let compute_identity = Identity::generate();
    let port = available_port();
    let route = Multiaddr::from_str(&format!("/ip4/127.0.0.1/tcp/{port}")).unwrap();

    let robot_config = AukiPeerConfig::new(
        "http://127.0.0.1:9",
        "dataset-facade-test",
        temp.path().join("robot"),
    )
    .unwrap()
    .direct_only()
    .with_listen_addresses([route.clone()])
    .unwrap()
    .with_advertised_direct_routes([route])
    .unwrap();
    let (robot, _robot_authority) = AukiPeer::start_external(
        robot_identity.clone(),
        authority(&robot_identity, domain_id, "robot"),
        robot_config,
    )
    .await
    .unwrap();
    let robot_dataset =
        P2pDatasetAdapter::new(robot.protocol_context(), DatasetRoutePolicy::DirectOnly).unwrap();
    let server = robot_dataset.start_serving().await.unwrap();

    let compute_config = AukiPeerConfig::new(
        "http://127.0.0.1:9",
        "dataset-facade-test",
        temp.path().join("compute"),
    )
    .unwrap()
    .direct_only();
    let (compute, _compute_authority) = AukiPeer::start_external(
        compute_identity.clone(),
        authority(&compute_identity, domain_id, "compute"),
        compute_config,
    )
    .await
    .unwrap();
    let compute_dataset =
        P2pDatasetAdapter::new(compute.protocol_context(), DatasetRoutePolicy::DirectOnly).unwrap();
    let compute_server = P2pDatasetAdapter::new(
        compute.protocol_context(),
        DatasetRoutePolicy::RelayRequired,
    )
    .unwrap();
    assert!(matches!(
        compute_server.start_serving().await,
        Err(P2pDatasetError::LocalPeerTypeMismatch { .. })
    ));

    let source = temp.path().join("source.bin");
    let destination = temp.path().join("destination.bin");
    let contents = b"one Domain, two facade-owned peers";
    fs::write(&source, contents).await.unwrap();
    let reference = robot_dataset
        .register_dataset(P2pDatasetRegistration {
            dataset_id: Uuid::new_v4().to_string(),
            name: "facade-transport.bin".into(),
            path: source.clone(),
            available_until: Utc::now() + chrono::Duration::minutes(5),
        })
        .await
        .unwrap();

    let mut wrong_domain = reference.clone();
    wrong_domain.domain_id = Uuid::new_v4();
    assert!(matches!(
        compute_dataset
            .fetch_dataset(&wrong_domain, &destination)
            .await,
        Err(P2pDatasetError::DomainMismatch)
    ));
    assert!(matches!(
        robot_dataset
            .fetch_dataset(&reference, &temp.path().join("robot-destination.bin"))
            .await,
        Err(P2pDatasetError::LocalPeerTypeMismatch { .. })
    ));

    compute_dataset
        .fetch_dataset(&reference, &destination)
        .await
        .unwrap();
    assert_eq!(fs::read(destination).await.unwrap(), contents);

    server.shutdown().await.unwrap();
    assert!(matches!(
        robot_dataset
            .register_dataset(P2pDatasetRegistration {
                dataset_id: Uuid::new_v4().to_string(),
                name: "after-shutdown.bin".into(),
                path: source,
                available_until: Utc::now() + chrono::Duration::minutes(5),
            })
            .await,
        Err(P2pDatasetError::RegistrationsStopped)
    ));
    compute.shutdown().await.unwrap();
    robot.shutdown().await.unwrap();
}

fn available_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn authority(identity: &Identity, domain_id: Uuid, peer_type: &str) -> ExternalAuthorityUpdate {
    let issued_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expiration = issued_at + P2P_TOKEN_TTL.as_secs();
    let claims = P2PAccessClaims {
        token_type: P2P_TOKEN_TYPE.into(),
        iss: P2P_TOKEN_ISSUER.into(),
        aud: vec![P2P_TOKEN_AUDIENCE.into()],
        sub: Uuid::new_v4().to_string(),
        organization_id: None,
        peer_type: Some(peer_type.into()),
        peer_id: identity.peer_id().to_string(),
        domain_ids: vec![domain_id.to_string()],
        scopes: vec![P2P_TOKEN_SCOPE.into()],
        application: None,
        iat: issued_at,
        nbf: None,
        exp: expiration,
    };
    let token = encode(
        &Header::new(Algorithm::ES256),
        &claims,
        &EncodingKey::from_ec_pem(TEST_DDS_PRIVATE_KEY).unwrap(),
    )
    .unwrap();
    ExternalAuthorityUpdate::new(
        domain_id,
        identity.peer_id(),
        DdsVerificationKeys::new(0, TEST_DDS_PUBLIC_KEY.to_vec(), None),
        SignedP2pCredential::new(token).unwrap(),
        Utc.timestamp_opt(expiration as i64, 0).unwrap(),
    )
}

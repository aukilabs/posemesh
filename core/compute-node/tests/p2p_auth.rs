#[path = "support/sdk.rs"]
#[allow(dead_code)]
mod sdk;

use std::time::Duration;

use async_trait::async_trait;
use auki_p2p::{
    Identity, P2PAccessClaims, PeerIdentityProof, PeerRole, P2P_TOKEN_AUDIENCE, P2P_TOKEN_ISSUER,
    P2P_TOKEN_SCOPE, P2P_TOKEN_TTL, P2P_TOKEN_TYPE,
};
use auki_sdk::{AukiPeer, AukiPeerConfig};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use httpmock::{prelude::*, Mock};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use p256::pkcs8::{DecodePublicKey, EncodePublicKey};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const CHALLENGE_PATH: &str = "/internal/v1/auth/p2p/challenge";
const VERIFY_PATH: &str = "/internal/v1/auth/p2p/verify";
const ROBOT_REGISTER_PATH: &str = "/internal/v1/robots/register";
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

fn binding_mocks<'a>(
    server: &'a MockServer,
    identity: &PeerIdentityProof,
    base_token: &str,
    bound_token: &str,
    challenge_id: &str,
    challenge_bytes: &[u8],
) -> (Mock<'a>, Mock<'a>) {
    let peer_id = identity.peer_id().to_string();
    let public_key = URL_SAFE_NO_PAD.encode(identity.public_key_protobuf());
    let challenge = URL_SAFE_NO_PAD.encode(challenge_bytes);
    let signature = URL_SAFE_NO_PAD.encode(identity.sign_challenge(challenge_bytes).unwrap());
    let expires_at = bound_token
        .split('.')
        .nth(1)
        .and_then(|claims| URL_SAFE_NO_PAD.decode(claims).ok())
        .and_then(|claims| serde_json::from_slice::<serde_json::Value>(&claims).ok())
        .and_then(|claims| claims["exp"].as_i64())
        .and_then(|expiry| DateTime::from_timestamp(expiry, 0))
        .unwrap_or_else(|| Utc::now() + chrono::Duration::minutes(10));
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
                "peer_id": identity.peer_id().to_string(),
                "access_token": bound_token,
                "access_expires_at": expires_at,
            }));
    });
    (challenge_mock, verify_mock)
}

#[tokio::test]
async fn robot_engine_supports_application_protocols_and_ordered_shutdown() {
    use futures::{AsyncReadExt, AsyncWriteExt};
    use posemesh_compute_node::{
        config::{P2pPrivateKey, RobotNodeConfig},
        engine::{run_robot_node_with_shutdowns, RunnerComposition, RunnerRegistry},
    };
    use tokio_util::sync::CancellationToken;

    struct IdleRunner;
    #[async_trait]
    impl compute_runner_api::Runner for IdleRunner {
        fn capability(&self) -> &'static str {
            "/test/idle/v1"
        }
        async fn run(&self, _: compute_runner_api::TaskCtx<'_>) -> anyhow::Result<()> {
            Ok(())
        }
    }

    let server = MockServer::start();
    let _keys = verification_keys_mock(&server, 1, TEST_DDS_PUBLIC_KEY, None);
    let identity = Identity::generate();
    let domain_id = Uuid::new_v4();
    let (token, expires_at) = signed_robot_p2p_token(&identity, domain_id);
    let robot_id = Uuid::new_v4();
    let base = sdk::robot_token(
        &server.base_url(),
        robot_id,
        domain_id,
        expires_at,
        "base",
        None,
    );
    let bound = sdk::robot_token(
        &server.base_url(),
        robot_id,
        domain_id,
        expires_at,
        "bound",
        Some(identity.peer_id().to_string()),
    );
    let _register = server.mock(|when, then| {
        when.method(POST).path(ROBOT_REGISTER_PATH);
        then.status(200).json_body(json!({
            "robot_id": robot_id, "access_token": base,
            "access_expires_at": expires_at,
        }));
    });
    let (_challenge, _verify) = binding_mocks(
        &server,
        &identity.proof(),
        &base,
        &bound,
        "robot-proof",
        b"robot proof",
    );
    let _exchange = server.mock(|when, then| {
        when.method(POST).path(ROBOT_P2P_TOKEN_PATH);
        then.status(200).json_body(json!({
            "p2p_access_token": token, "p2p_access_expires_at": expires_at,
        }));
    });
    let _no_work = server.mock(|when, then| {
        when.method(GET).path("/tasks");
        then.status(204);
    });
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!(
        "/ip4/127.0.0.1/tcp/{}",
        listener.local_addr().unwrap().port()
    );
    drop(listener);
    let mut cfg = RobotNodeConfig::new(
        server.base_url().parse().unwrap(),
        server.base_url().parse().unwrap(),
        "robot-test-credential",
    )
    .unwrap();
    cfg.set_audience(format!("{}/robots", server.base_url()))
        .unwrap();
    cfg.auki_p2p_enabled = true;
    cfg.set_relay_config(None).unwrap();
    cfg.auki_p2p_listen_multiaddrs = vec![address.clone()];
    cfg.auki_p2p_advertised_multiaddrs = vec![address.clone()];
    cfg.set_p2p_private_key(Some(
        P2pPrivateKey::from_protobuf_encoding(identity.to_protobuf_encoding().unwrap()).unwrap(),
    ));
    cfg.poll_backoff_ms_min = 10;
    cfg.poll_backoff_ms_max = 10;

    let (handle_tx, handle_rx) = tokio::sync::oneshot::channel();
    let runners = RunnerComposition::with_protocols(move |handle| {
        assert!(handle.get().is_err(), "composition precedes peer startup");
        assert!(handle_tx.send(handle).is_ok());
        RunnerRegistry::new().register(IdleRunner)
    });
    let shutdown = CancellationToken::new();
    let _shutdown_guard = shutdown.clone().drop_guard();
    let engine = tokio::spawn(run_robot_node_with_shutdowns(
        cfg,
        runners,
        shutdown.clone(),
        CancellationToken::new(),
    ));
    let context = tokio::time::timeout(Duration::from_secs(5), async {
        let handle = handle_rx.await.unwrap();
        loop {
            if let Ok(context) = handle.get() {
                break context;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Robot peer startup");
    assert_eq!(context.peer_id(), identity.peer_id());
    assert_eq!(context.domain_id(), domain_id);
    let registration = context
        .protocols()
        .register(
            auki_sdk::AukiProtocolSpec::new("/test/application/1", 1, 16).unwrap(),
            |mut stream| async move {
                let mut request = [0; 4];
                stream.read_exact(&mut request).await.unwrap();
                assert_eq!(&request, b"ping");
                stream.write_all(b"pong").await.unwrap();
                stream.close().await.unwrap();
            },
        )
        .unwrap();

    let remote_identity = Identity::generate();
    let (token, expires_at) = signed_p2p_token(
        &remote_identity,
        domain_id,
        PeerRole::Compute,
        Uuid::new_v4(),
    );
    let material = auki_auth::machine::p2p::DdsP2pClient::new(
        server.base_url().parse().unwrap(),
        Duration::from_secs(2),
    )
    .unwrap()
    .authority_material(&remote_identity.proof(), domain_id, &token, expires_at)
    .await
    .unwrap();
    let update = auki_sdk::ExternalAuthorityUpdate::new(
        material.domain_id,
        material.peer_id,
        material.verification_keys,
        material.credential,
        material.expires_at,
    );
    let (remote, _) = AukiPeer::start_external(
        remote_identity,
        update,
        AukiPeerConfig::new(server.base_url())
            .unwrap()
            .direct_only(),
    )
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut stream = remote
            .protocols()
            .open_exact(
                identity.peer_id(),
                address.parse().unwrap(),
                "/test/application/1",
            )
            .await
            .unwrap();
        stream.write_all(b"ping").await.unwrap();
        stream.flush().await.unwrap();
        let mut response = [0; 4];
        stream.read_exact(&mut response).await.unwrap();
        assert_eq!(&response, b"pong");
        stream.close().await.unwrap();
    })
    .await
    .expect("authenticated custom protocol exchange");

    shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(5), engine)
        .await
        .expect("Robot shutdown is bounded")
        .unwrap()
        .unwrap();
    assert!(
        context
            .protocols()
            .register(
                auki_sdk::AukiProtocolSpec::new("/test/after-shutdown/1", 1, 16).unwrap(),
                |_| async {},
            )
            .is_err(),
        "shutdown fences application protocol work"
    );
    registration.close().await.unwrap();
    remote.shutdown().await.unwrap();
}

#[tokio::test]
async fn compute_protocols_follow_each_lease_and_are_cleared_on_every_exit() {
    use auki_sdk::{AukiPeerProtocolContext, AukiProtocolSpec};
    use posemesh_compute_node::{
        config::{LogFormat, NodeConfig, P2pPrivateKey},
        engine::{run_node_with_shutdown, AukiProtocolsHandle, RunnerComposition, RunnerRegistry},
    };
    use tokio::sync::{mpsc, oneshot};
    use tokio_util::sync::CancellationToken;

    const CAPABILITY: &str = "/test/arbitrary-capability/v42";
    type Invocation = (Option<AukiPeerProtocolContext>, oneshot::Sender<bool>);
    struct ProbeRunner {
        protocols: AukiProtocolsHandle,
        started: mpsc::UnboundedSender<Invocation>,
    }
    #[async_trait]
    impl compute_runner_api::Runner for ProbeRunner {
        fn capability(&self) -> &'static str {
            CAPABILITY
        }

        async fn run(&self, ctx: compute_runner_api::TaskCtx<'_>) -> anyhow::Result<()> {
            let (release, result) = oneshot::channel();
            self.started
                .send((self.protocols.get().ok(), release))
                .unwrap();
            tokio::select! {
                result = result => {
                    anyhow::ensure!(result?, "requested runner failure");
                    Ok(())
                }
                _ = async {
                    while !ctx.ctrl.is_cancelled().await {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                } => anyhow::bail!("runner observed cancellation"),
            }
        }
    }

    fn assert_stopped(context: &AukiPeerProtocolContext) {
        assert!(
            context
                .protocols()
                .register(
                    AukiProtocolSpec::new("/test/stale-context/1", 1, 16).unwrap(),
                    |_| async {},
                )
                .is_err(),
            "a retained context must not register protocols after its task ends"
        );
    }

    async fn wait_until(condition: impl Fn() -> bool) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !condition() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("engine transition should be bounded");
    }

    let server = MockServer::start();
    let _keys = verification_keys_mock(&server, 1, TEST_DDS_PUBLIC_KEY, None);
    let identity = Identity::generate();
    let _nonce = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/auth/siwe/request");
        then.status(200).json_body(json!({
            "nonce": "nonce-123", "domain": "dds.example.test",
            "uri": "https://dds.example.test/login", "version": "1",
            "chainId": 1, "issuedAt": Utc::now().to_rfc3339(),
        }));
    });
    let _login = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/auth/siwe/verify");
        then.status(200).json_body(json!({
            "access_token": "compute-base",
            "access_expires_at": Utc::now() + chrono::Duration::hours(1),
        }));
    });
    let (_challenge, _verify) = binding_mocks(
        &server,
        &identity.proof(),
        "compute-base",
        "compute-bound",
        "compute-proof",
        b"compute proof",
    );
    let mut idle = server.mock(|when, then| {
        when.method(GET).path("/tasks");
        then.status(204);
    });
    let cfg = NodeConfig {
        dms_base_url: server.base_url().parse().unwrap(),
        dds_base_url: Some(server.base_url().parse().unwrap()),
        node_version: "test".into(),
        request_timeout_secs: 2,
        reg_secret: Some("test-registration-secret".into()),
        secp256k1_privhex: Some("01".repeat(32)),
        heartbeat_jitter_ms: 0,
        heartbeat_min_ratio: 0.01,
        heartbeat_max_ratio: 0.01,
        poll_backoff_ms_min: 10,
        poll_backoff_ms_max: 10,
        token_safety_ratio: 0.75,
        token_reauth_max_retries: 0,
        token_reauth_jitter_ms: 0,
        auki_p2p_enabled: true,
        auki_p2p_listen_multiaddrs: Vec::new(),
        auki_p2p_advertised_multiaddrs: Vec::new(),
        auki_p2p_private_key: Some(
            P2pPrivateKey::from_protobuf_encoding(identity.to_protobuf_encoding().unwrap())
                .unwrap(),
        ),
        register_interval_secs: None,
        register_max_retry: None,
        max_concurrency: 1,
        log_format: LogFormat::Json,
        enable_noop: false,
        noop_sleep_secs: 0,
    };
    let _register = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/nodes/register-wallet");
        then.status(200);
    });
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (handle_tx, handle_rx) = oneshot::channel();
    let runners = RunnerComposition::with_protocols(move |protocols| {
        assert!(protocols.get().is_err(), "composition precedes any task");
        assert!(handle_tx.send(protocols.clone()).is_ok());
        RunnerRegistry::new().register(ProbeRunner {
            protocols,
            started: started_tx,
        })
    });
    let shutdown = CancellationToken::new();
    let _shutdown_guard = shutdown.clone().drop_guard();
    let mut engine = tokio::spawn(run_node_with_shutdown(cfg, runners, shutdown));
    let handle = tokio::time::timeout(Duration::from_secs(5), handle_rx)
        .await
        .unwrap()
        .unwrap();
    let mut old_contexts = Vec::new();

    for outcome in ["complete", "fail", "cancel", "http-only", "abort"] {
        let task_id = Uuid::new_v4();
        let domain_id = Uuid::new_v4();
        let (token, expiry) =
            signed_p2p_token(&identity, domain_id, PeerRole::Compute, Uuid::new_v4());
        let mut lease = json!({
            "access_token": sdk::data_token(&server.base_url(), domain_id, Utc::now() + chrono::Duration::seconds(30), "task"),
            "access_token_expires_at": Utc::now() + chrono::Duration::seconds(30),
            "lease_expires_at": Utc::now() + chrono::Duration::seconds(30),
            "domain_id": domain_id, "domain_server_url": server.base_url(),
            "cancel": false, "status": "running",
            "task": {
                "id": task_id, "capability": CAPABILITY, "mode": "dedicated",
                "inputs_cids": [], "outputs_prefix": "test/",
            },
        });
        if outcome != "http-only" {
            lease["p2p_access_token"] = json!(token);
            lease["p2p_access_token_expires_at"] = json!(expiry);
        }
        let heartbeat_path = format!("/tasks/{task_id}/heartbeat");
        let mut heartbeat = server.mock(|when, then| {
            when.method(POST)
                .path(&heartbeat_path)
                .header("authorization", "Bearer compute-bound");
            then.status(200).json_body(lease.clone());
        });
        let complete = server.mock(|when, then| {
            when.method(POST).path(format!("/tasks/{task_id}/complete"));
            then.status(200);
        });
        let fail = server.mock(|when, then| {
            when.method(POST).path(format!("/tasks/{task_id}/fail"));
            then.status(200);
        });
        idle.delete();
        let mut claim = server.mock(|when, then| {
            when.method(GET)
                .path("/tasks")
                .header("authorization", "Bearer compute-bound");
            then.status(200).json_body(lease.clone());
        });
        let (context, release) = tokio::time::timeout(Duration::from_secs(5), started_rx.recv())
            .await
            .unwrap_or_else(|_| panic!("runner dispatch timed out for {outcome}"))
            .unwrap();
        claim.delete();
        idle = server.mock(|when, then| {
            when.method(GET).path("/tasks");
            then.status(204);
        });
        assert_eq!(context.is_some(), outcome != "http-only");
        for old in &old_contexts {
            assert_stopped(old);
        }
        if let Some(context) = context {
            assert_eq!(context.peer_id(), identity.peer_id());
            assert_eq!(context.domain_id(), domain_id);
            assert_eq!(handle.get().unwrap().domain_id(), domain_id);
            let registration = context
                .protocols()
                .register(
                    AukiProtocolSpec::new("/test/current-context/1", 1, 16).unwrap(),
                    |_| async {},
                )
                .expect("every capability gets a usable protocol surface");
            registration.close().await.unwrap();
            old_contexts.push(context);
        } else {
            assert!(
                handle.get().is_err(),
                "HTTP-only tasks cannot reuse old authority"
            );
        }

        match outcome {
            "cancel" => {
                heartbeat.delete();
                lease["cancel"] = json!(true);
                lease["p2p_access_token"] = serde_json::Value::Null;
                lease["p2p_access_token_expires_at"] = serde_json::Value::Null;
                let _cancel = server.mock(|when, then| {
                    when.method(POST).path(&heartbeat_path);
                    then.status(200).json_body(lease.clone());
                });
                wait_until(|| handle.get().is_err()).await;
                complete.assert_hits(0);
                fail.assert_hits(0);
            }
            "abort" => {
                engine.abort();
                assert!((&mut engine).await.unwrap_err().is_cancelled());
                assert!(
                    handle.get().is_err(),
                    "dropping the cycle clears the shared handle"
                );
                complete.assert_hits(0);
                fail.assert_hits(0);
            }
            _ => {
                release.send(outcome == "complete").unwrap();
                let terminal = if outcome == "complete" {
                    &complete
                } else {
                    &fail
                };
                wait_until(|| terminal.hits() == 1).await;
                assert!(
                    handle.get().is_err(),
                    "terminal reporting follows peer shutdown"
                );
            }
        }
        for old in &old_contexts {
            assert_stopped(old);
        }
    }
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

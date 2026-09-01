use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use auki_p2p::{
    P2PAccessClaims, P2P_TOKEN_AUDIENCE, P2P_TOKEN_ISSUER, P2P_TOKEN_SCOPE, P2P_TOKEN_TTL,
    P2P_TOKEN_TYPE,
};
use auki_sdk::{
    AukiPeer, AukiPeerConfig, AukiRelayConfig, AukiRelayMode, DdsVerificationKeys,
    ExternalAuthorityUpdate, Identity, SignedP2pCredential,
};
use axum::{
    extract::{Path, State},
    http::{
        header::{ACCEPT, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, LOCATION},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{SecondsFormat, TimeZone, Utc};
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use libp2p::{
    noise, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, PeerId, StreamProtocol, SwarmBuilder,
};
use libp2p_stream::{Behaviour as StreamBehaviour, IncomingStreams};
use posemesh_p2p_dataset::{DatasetRoutePolicy, P2pDatasetAdapter, P2pDatasetRegistration};
use serde_json::json;
use tempfile::TempDir;
use tokio::{fs, net::TcpListener, sync::mpsc, task::JoinHandle};
use uuid::Uuid;

const SOURCE_ADMISSION_PROTOCOL: &str = "/auki-p2p/relay-auth/1";
const CIRCUIT_DURATION: Duration = Duration::from_secs(15 * 60);
// A dataset reserves 1 MiB of protocol overhead in addition to its contents.
const CIRCUIT_BYTES: u64 = 2 * 1024 * 1024;
const TEST_TIMEOUT: Duration = Duration::from_secs(30);

const TEST_DDS_PRIVATE_KEY: &[u8] = br#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQggm4twpf4y/yNNw/k
fqecEEl4zBTwZdRDFUFp/fSxV8qhRANCAARUxrDWJ0AtEGTAYZ4412VPHqMCKoPw
UphDkcOIk7SODsKwUvTIiUr11NbXBJmbBRfhERczsuK4PVha5eg0fVqo
-----END PRIVATE KEY-----"#;
const TEST_DDS_PUBLIC_KEY: &[u8] = br#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEVMaw1idALRBkwGGeONdlTx6jAiqD
8FKYQ5HDiJO0jg7CsFL0yIlK9dTW1wSZmwUX4REXM7LiuD1YWuXoNH1aqA==
-----END PUBLIC KEY-----"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn facade_allocates_and_uses_a_real_circuit_route() {
    tokio::time::timeout(TEST_TIMEOUT, relay_transfer())
        .await
        .expect("facade relay transfer timed out");
}

async fn relay_transfer() {
    let mut relay = RelayHarness::start().await;
    let booking_id = Uuid::new_v4();
    let snapshot = ready_booking(booking_id, relay.peer_id, relay.port);
    let dms = FakeDms::start(booking_id, snapshot).await;

    let temp = TempDir::new().unwrap();
    let domain_id = Uuid::new_v4();
    let robot_identity = Identity::generate();
    let robot_config = AukiPeerConfig::new(dms.base_url())
        .unwrap()
        .with_relay(
            AukiRelayConfig::new(
                AukiRelayMode::Public,
                1,
                CIRCUIT_DURATION,
                Duration::from_secs(1),
            )
            .unwrap(),
        )
        .unwrap();
    let (robot, _robot_authority) = AukiPeer::start_external(
        robot_identity.clone(),
        authority(&robot_identity, domain_id, "robot"),
        robot_config,
    )
    .await
    .unwrap();

    assert_eq!(relay.reservations.recv().await, Some(robot.peer_id()));
    let route_snapshot = robot.protocol_context().routes().snapshot().unwrap();
    assert!(route_snapshot.direct_routes.is_empty());
    assert_eq!(route_snapshot.relay_routes.len(), 1);
    let relay_route = &route_snapshot.relay_routes[0];
    assert_eq!(relay_route.relay_peer_id, relay.peer_id);
    assert_eq!(relay_route.limits.duration(), CIRCUIT_DURATION);
    assert_eq!(relay_route.limits.data_bytes_per_direction(), CIRCUIT_BYTES);
    let published_tcp_route = relay_route.routes.tcp().to_string();
    let published_wss_route = relay_route.routes.wss().to_string();
    assert!(!published_tcp_route.contains("/wss/"));
    assert!(published_wss_route.contains("/wss/"));
    assert_ne!(published_tcp_route, published_wss_route);

    let robot_dataset =
        P2pDatasetAdapter::new(robot.protocol_context(), DatasetRoutePolicy::RelayRequired)
            .unwrap();
    let server = robot_dataset.start_serving().await.unwrap();
    let source = temp.path().join("source.bin");
    let destination = temp.path().join("destination.bin");
    let contents = b"facade-owned relay route";
    fs::write(&source, contents).await.unwrap();
    let reference = robot_dataset
        .register_dataset(P2pDatasetRegistration {
            dataset_id: Uuid::new_v4().to_string(),
            name: "relay-facade.bin".into(),
            path: source,
            available_until: Utc::now() + chrono::Duration::minutes(2),
        })
        .await
        .unwrap();
    assert_eq!(
        reference.multiaddrs,
        vec![published_tcp_route, published_wss_route]
    );
    assert!(reference
        .multiaddrs
        .iter()
        .all(|route| route.contains("/p2p-circuit/")));

    let compute_identity = Identity::generate();
    let compute_config = AukiPeerConfig::new("http://127.0.0.1:9")
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
    compute_dataset
        .fetch_dataset(&reference, &destination)
        .await
        .unwrap();
    assert_eq!(fs::read(destination).await.unwrap(), contents);
    assert_eq!(relay.admissions.recv().await, Some(compute.peer_id()));
    assert_eq!(
        relay.circuits.recv().await,
        Some((compute.peer_id(), robot.peer_id()))
    );

    server.shutdown().await.unwrap();
    compute.shutdown().await.unwrap();
    dms.wait_for_reconciliation().await;
    robot.shutdown().await.unwrap();
    relay.shutdown().await;
    assert_eq!(dms.create_calls(), 1);
    assert!(dms.active_calls() >= 2);
    assert_eq!(dms.delete_calls(), 1);
    dms.shutdown().await;
}

fn ready_booking(booking_id: Uuid, relay_peer_id: PeerId, port: u16) -> serde_json::Value {
    let created_at = Utc::now();
    json!({
        "booking_id": booking_id,
        "mode": "public",
        "state": "active",
        "relay_count": 1,
        "requested_duration_seconds": CIRCUIT_DURATION.as_secs(),
        "requested_until": created_at + chrono::Duration::minutes(15),
        "authority_expires_at": created_at + chrono::Duration::minutes(5),
        "assigned_count": 1,
        "provider_ready_count": 1,
        "unfilled_count": 0,
        "created_at": created_at,
        "slots": [{
            "slot_id": Uuid::new_v4(),
            "slot_index": 0,
            "state": "ready",
            "assignment_id": Uuid::new_v4(),
            "reservation_epoch": Uuid::new_v4(),
            "provider_peer_id": relay_peer_id.to_string(),
            "provider_base_addresses": [
                format!("/dns4/relay.127-0-0-1.nip.io/tcp/{port}/p2p/{relay_peer_id}"),
                format!("/dns4/relay.127-0-0-1.nip.io/tcp/{port}/wss/p2p/{relay_peer_id}")
            ],
            "limits": {
                "duration_seconds": CIRCUIT_DURATION.as_secs(),
                "data_bytes_per_direction": CIRCUIT_BYTES
            },
            "provider_lease_expires_at": created_at + chrono::Duration::minutes(4)
        }]
    })
}

struct FakeDmsState {
    booking_id: Uuid,
    snapshot: serde_json::Value,
    created: AtomicBool,
    active_calls: AtomicUsize,
    create_calls: AtomicUsize,
    delete_calls: AtomicUsize,
}

struct FakeDms {
    base_url: String,
    state: Arc<FakeDmsState>,
    task: Option<JoinHandle<()>>,
}

impl FakeDms {
    async fn start(booking_id: Uuid, snapshot: serde_json::Value) -> Self {
        let state = Arc::new(FakeDmsState {
            booking_id,
            snapshot,
            created: AtomicBool::new(false),
            active_calls: AtomicUsize::new(0),
            create_calls: AtomicUsize::new(0),
            delete_calls: AtomicUsize::new(0),
        });
        let app = Router::new()
            .route("/relay-bookings/active", get(fake_active_booking))
            .route("/relay-bookings", post(fake_create_booking))
            .route("/relay-bookings/:booking_id", delete(fake_delete_booking))
            .with_state(state.clone());
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            base_url: format!("http://{address}"),
            state,
            task: Some(task),
        }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn active_calls(&self) -> usize {
        self.state.active_calls.load(Ordering::SeqCst)
    }

    fn create_calls(&self) -> usize {
        self.state.create_calls.load(Ordering::SeqCst)
    }

    fn delete_calls(&self) -> usize {
        self.state.delete_calls.load(Ordering::SeqCst)
    }

    async fn wait_for_reconciliation(&self) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while self.active_calls() < 2 {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("relay booking was not reconciled through the active endpoint");
    }

    async fn shutdown(mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            let result = task.await;
            assert!(result.is_ok() || result.unwrap_err().is_cancelled());
        }
    }
}

impl Drop for FakeDms {
    fn drop(&mut self) {
        if let Some(task) = self.task.as_ref() {
            task.abort();
        }
    }
}

async fn fake_active_booking(
    State(state): State<Arc<FakeDmsState>>,
    headers: HeaderMap,
) -> Response {
    assert_common_dms_headers(&headers);
    state.active_calls.fetch_add(1, Ordering::SeqCst);
    if state.created.load(Ordering::SeqCst) {
        json_response(StatusCode::OK, state.snapshot.clone(), None)
    } else {
        empty_response(StatusCode::NO_CONTENT)
    }
}

async fn fake_create_booking(
    State(state): State<Arc<FakeDmsState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    assert_common_dms_headers(&headers);
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .expect("create request omitted its valid Idempotency-Key");
    assert!(!idempotency_key.is_empty());
    assert!(idempotency_key.len() <= 128);
    assert!(idempotency_key.bytes().all(|byte| byte.is_ascii_graphic()));
    assert_eq!(
        headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/json")
    );
    assert_eq!(
        body,
        json!({
            "mode": "public",
            "requested_duration_seconds": CIRCUIT_DURATION.as_secs(),
            "relay_count": 1
        })
    );
    assert!(!state.created.swap(true, Ordering::SeqCst));
    state.create_calls.fetch_add(1, Ordering::SeqCst);
    json_response(
        StatusCode::CREATED,
        state.snapshot.clone(),
        Some(format!("/relay-bookings/{}", state.booking_id)),
    )
}

async fn fake_delete_booking(
    Path(booking_id): Path<Uuid>,
    State(state): State<Arc<FakeDmsState>>,
    headers: HeaderMap,
) -> Response {
    assert_common_dms_headers(&headers);
    assert_eq!(booking_id, state.booking_id);
    assert!(state.created.load(Ordering::SeqCst));
    state.delete_calls.fetch_add(1, Ordering::SeqCst);
    empty_response(StatusCode::NO_CONTENT)
}

fn assert_common_dms_headers(headers: &HeaderMap) {
    let authorization = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .expect("relay lifecycle request omitted bearer authorization");
    assert!(authorization.starts_with("Bearer "));
    assert!(authorization.len() > "Bearer ".len());
    assert_eq!(
        headers.get(ACCEPT).and_then(|value| value.to_str().ok()),
        Some("application/json")
    );
}

fn empty_response(status: StatusCode) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    (status, headers).into_response()
}

fn json_response(
    status: StatusCode,
    body: serde_json::Value,
    location: Option<String>,
) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    if let Some(location) = location {
        headers.insert(LOCATION, HeaderValue::from_str(&location).unwrap());
    }
    (status, headers, Json(body)).into_response()
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

#[derive(NetworkBehaviour)]
struct RelayBehaviour {
    relay: relay::Behaviour,
    streams: StreamBehaviour,
}

struct RelayHarness {
    peer_id: PeerId,
    port: u16,
    admissions: mpsc::Receiver<PeerId>,
    reservations: mpsc::Receiver<PeerId>,
    circuits: mpsc::Receiver<(PeerId, PeerId)>,
    swarm_task: Option<JoinHandle<()>>,
    admission_task: Option<JoinHandle<()>>,
}

impl RelayHarness {
    async fn start() -> Self {
        let identity = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = identity.public().to_peer_id();
        let streams = StreamBehaviour::new();
        let mut control = streams.new_control();
        let incoming = control
            .accept(StreamProtocol::new(SOURCE_ADMISSION_PROTOCOL))
            .unwrap();
        let config = relay::Config {
            reservation_duration: Duration::from_secs(10 * 60),
            reservation_rate_limiters: Vec::new(),
            max_circuit_duration: CIRCUIT_DURATION,
            max_circuit_bytes: CIRCUIT_BYTES,
            circuit_src_rate_limiters: Vec::new(),
            ..Default::default()
        };
        let mut swarm = SwarmBuilder::with_existing_identity(identity)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )
            .unwrap()
            .with_behaviour(move |_| RelayBehaviour {
                relay: relay::Behaviour::new(peer_id, config),
                streams,
            })
            .unwrap()
            .build();
        swarm
            .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();
        let address = loop {
            if let SwarmEvent::NewListenAddr { address, .. } = swarm.select_next_some().await {
                break address;
            }
        };
        swarm.add_external_address(address.clone());
        let port = address
            .iter()
            .find_map(|protocol| match protocol {
                libp2p::multiaddr::Protocol::Tcp(port) => Some(port),
                _ => None,
            })
            .unwrap();

        let (reservation_tx, reservations) = mpsc::channel(2);
        let (circuit_tx, circuits) = mpsc::channel(2);
        let swarm_task = tokio::spawn(async move {
            while let Some(event) = swarm.next().await {
                if let SwarmEvent::Behaviour(RelayBehaviourEvent::Relay(event)) = event {
                    match event {
                        relay::Event::ReservationReqAccepted { src_peer_id, .. } => {
                            let _ = reservation_tx.send(src_peer_id).await;
                        }
                        relay::Event::CircuitReqAccepted {
                            src_peer_id,
                            dst_peer_id,
                        } => {
                            let _ = circuit_tx.send((src_peer_id, dst_peer_id)).await;
                        }
                        _ => {}
                    }
                }
            }
        });
        let (admission_tx, admissions) = mpsc::channel(2);
        let admission_task = tokio::spawn(serve_source_admission(incoming, admission_tx));
        Self {
            peer_id,
            port,
            admissions,
            reservations,
            circuits,
            swarm_task: Some(swarm_task),
            admission_task: Some(admission_task),
        }
    }

    async fn shutdown(mut self) {
        if let Some(task) = self.swarm_task.take() {
            task.abort();
            let _ = task.await;
        }
        if let Some(task) = self.admission_task.take() {
            task.abort();
            let _ = task.await;
        }
    }
}

impl Drop for RelayHarness {
    fn drop(&mut self) {
        if let Some(task) = self.swarm_task.as_ref() {
            task.abort();
        }
        if let Some(task) = self.admission_task.as_ref() {
            task.abort();
        }
    }
}

async fn serve_source_admission(mut incoming: IncomingStreams, admissions: mpsc::Sender<PeerId>) {
    while let Some((source_peer_id, mut stream)) = incoming.next().await {
        let mut length = [0; 4];
        stream.read_exact(&mut length).await.unwrap();
        let frame_size = u32::from_be_bytes(length) as usize;
        assert!((1..=64 * 1024).contains(&frame_size));
        let mut frame = vec![0; frame_size];
        stream.read_exact(&mut frame).await.unwrap();
        let _: serde_json::Value = serde_json::from_slice(&frame).unwrap();
        admissions.send(source_peer_id).await.unwrap();

        let response = serde_json::to_vec(&json!({
            "accepted": true,
            "accepted_until": (Utc::now() + chrono::Duration::seconds(20))
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        }))
        .unwrap();
        stream
            .write_all(&(response.len() as u32).to_be_bytes())
            .await
            .unwrap();
        stream.write_all(&response).await.unwrap();
        stream.flush().await.unwrap();
    }
}

//! Real Circuit Relay v2 coverage for the P2P dataset consumer.
//!
//! Production-shaped relay FQDNs resolve through a process-local UDP server,
//! keeping the test independent of host DNS and the public network.

use std::{
    io::ErrorKind,
    net::{Ipv4Addr, SocketAddr, TcpListener, UdpSocket},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use auki_p2p::{
    ApplicationProtocol, AuthenticatedStream, DdsTokenVerifier, ExpectedRelayLimits, Identity,
    Multiaddr, Node, P2PAccessClaims, PeerId, PeerRole, Protocol, RelayProvider,
    SessionRequirements, P2P_TOKEN_AUDIENCE, P2P_TOKEN_ISSUER, P2P_TOKEN_SCOPE, P2P_TOKEN_TTL,
    P2P_TOKEN_TYPE,
};
use chrono::{DateTime, SecondsFormat, Utc};
use compute_runner_api::{P2pDatasetReference, P2pDatasetRegistration, P2P_DATASET_SCHEMA};
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use hickory_resolver::config::{
    NameServerConfig, Protocol as DnsProtocol, ResolverConfig, ResolverOpts,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use libp2p::{
    noise, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, StreamProtocol, SwarmBuilder,
};
use libp2p_stream::{Behaviour as StreamBehaviour, IncomingStreams};
use posemesh_compute_node::{
    dds::p2p::P2pCredentialStore,
    p2p_dataset::{
        ConfirmedRelayRoute, DatasetRoutePolicy, P2pDatasetAdapter, RelayRouteFence,
        DATASET_PROTOCOL,
    },
};
use prometheus_client::encoding::text::encode as encode_metrics;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tempfile::TempDir;
use tokio::{fs, sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const SOURCE_ADMISSION_PROTOCOL: &str = "/auki-p2p/relay-auth/1";
const CIRCUIT_DURATION: Duration = Duration::from_secs(15 * 60);
const CIRCUIT_DATA_BYTES: u64 = 8 * 1024 * 1024;
const TEST_TIMEOUT: Duration = Duration::from_secs(30);
const DATASET_BYTES: usize = 256 * 1024 + 333;

type AttemptLog = Arc<Mutex<Vec<&'static str>>>;

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
async fn direct_failure_then_relay_a_denial_then_relay_b_streams_the_full_dataset() {
    let dns = TestDns::start();
    let attempts = AttemptLog::default();
    let mut relay_a = RelayHarness::start("a", false, attempts.clone()).await;
    let mut relay_b = RelayHarness::start("b", true, attempts.clone()).await;
    let domain_id = Uuid::new_v4();

    let robot = node(&dns);
    let robot_credentials = P2pCredentialStore::new(robot.clone());
    install_current_token(&robot_credentials, &robot, PeerRole::Robot, domain_id).await;

    let compute = node(&dns);
    let compute_credentials = P2pCredentialStore::new(compute.clone());
    let compute_token =
        install_current_token(&compute_credentials, &compute, PeerRole::Compute, domain_id).await;

    // Copied authorization cannot authenticate a different Noise identity.
    let impostor = node(&dns);
    assert!(matches!(
        impostor.install_token(compute_token.clone()).await,
        Err(auki_p2p::Error::PeerIdMismatch { .. })
    ));
    must_succeed(impostor.shutdown()).await;

    let robot_adapter = P2pDatasetAdapter::new_with_route_policy(
        robot.clone(),
        robot_credentials,
        Vec::new(),
        DatasetRoutePolicy::RelayRequired,
    )
    .unwrap();
    let shutdown = CancellationToken::new();
    let dataset_server = robot_adapter
        .start_serving(domain_id, &shutdown)
        .await
        .unwrap();

    let provider_a = relay_a.provider();
    let provider_b = relay_b.provider();
    let reservation_a = must_succeed(robot.start_relay_reservation(provider_a.clone())).await;
    let reservation_b = must_succeed(robot.start_relay_reservation(provider_b.clone())).await;
    let snapshot_a = must_succeed(robot.wait_relay_reservation(reservation_a)).await;
    let snapshot_b = must_succeed(robot.wait_relay_reservation(reservation_b)).await;
    let route_a = snapshot_a.publishable_route().unwrap().clone();
    let route_b = snapshot_b.publishable_route().unwrap().clone();

    assert_eq!(route_a, route_from_provider(&provider_a, robot.peer_id()));
    assert_eq!(route_b, route_from_provider(&provider_b, robot.peer_id()));
    assert_ne!(provider_a.relay_peer_id(), provider_b.relay_peer_id());
    assert_eq!(
        timeout(relay_a.reservations.recv()).await,
        Some(robot.peer_id())
    );
    assert_eq!(
        timeout(relay_b.reservations.recv()).await,
        Some(robot.peer_id())
    );

    publish_route(&robot_adapter, 1, reservation_a, route_a.clone());
    publish_route(&robot_adapter, 2, reservation_b, route_b.clone());

    let temp = TempDir::new().unwrap();
    let source_bytes = zip_like_bytes(DATASET_BYTES);
    assert!(source_bytes.len() > 128 * 1024);
    let source_path = temp.path().join("source.zip");
    fs::write(&source_path, &source_bytes).await.unwrap();
    let reference = robot_adapter
        .register_dataset(P2pDatasetRegistration {
            dataset_id: "relay-e2e".into(),
            domain_id,
            name: "relay-e2e.zip".into(),
            path: source_path,
            available_until: Utc::now() + chrono::Duration::minutes(10),
        })
        .await
        .unwrap();
    assert_eq!(
        reference.multiaddrs,
        vec![route_a.to_string(), route_b.to_string()]
    );

    let compute_adapter = P2pDatasetAdapter::new(compute.clone(), compute_credentials, Vec::new());

    // A reachable direct candidate must win without sending source-admission
    // or circuit requests to either captured relay.
    let mut direct_reference = reference.clone();
    direct_reference.multiaddrs.insert(
        0,
        must_succeed(robot.first_listen_address()).await.to_string(),
    );
    let direct_destination = temp.path().join("downloaded-direct.zip");
    timeout(compute_adapter.fetch_dataset(&direct_reference, &direct_destination))
        .await
        .unwrap();
    assert_eq!(fs::read(&direct_destination).await.unwrap(), source_bytes);
    assert_no_event(
        &mut relay_a.admissions,
        "relay A admission during direct fetch",
    )
    .await;
    assert_no_event(
        &mut relay_b.admissions,
        "relay B admission during direct fetch",
    )
    .await;
    assert_no_event(&mut relay_a.circuits, "relay A circuit during direct fetch").await;
    assert_no_event(&mut relay_b.circuits, "relay B circuit during direct fetch").await;

    // Prompt 9 must fail this instrumented direct candidate, then try A
    // (admission denied) before B.
    let mut relay_reference = reference;
    let direct_probe = DirectFailureProbe::start(attempts.clone());
    relay_reference.multiaddrs.insert(0, direct_probe.route());
    let relay_destination = temp.path().join("downloaded-relay.zip");
    let relay_b_bytes_before = relay_b.transport_bytes();
    timeout(compute_adapter.fetch_dataset(&relay_reference, &relay_destination))
        .await
        .unwrap();

    assert!(
        direct_probe.was_connected(),
        "the direct candidate was not attempted before relay fallback"
    );
    assert_eq!(fs::read(&relay_destination).await.unwrap(), source_bytes);
    assert_no_partial_files(temp.path()).await;

    assert_admission(
        timeout(relay_a.admissions.recv()).await.unwrap(),
        compute.peer_id(),
        robot.peer_id(),
        domain_id,
        &compute_token,
    );
    assert_admission(
        timeout(relay_b.admissions.recv()).await.unwrap(),
        compute.peer_id(),
        robot.peer_id(),
        domain_id,
        &compute_token,
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(100), relay_a.circuits.recv())
            .await
            .is_err(),
        "relay A opened a circuit despite denying source admission"
    );
    let circuit_b = timeout(relay_b.circuits.recv()).await.unwrap();
    assert_eq!(circuit_b.source_peer_id, compute.peer_id());
    assert_eq!(circuit_b.target_peer_id, robot.peer_id());
    assert_eq!(
        attempts.lock().unwrap().as_slice(),
        ["direct", "a", "b"],
        "candidate attempts were not globally direct-first and reference-stable"
    );
    let relay_b_bytes = relay_b.transport_bytes() - relay_b_bytes_before;
    assert!(
        relay_b_bytes > source_bytes.len() as u64,
        "relay B transport recorded only {relay_b_bytes} bytes for a {}-byte dataset",
        source_bytes.len()
    );

    dataset_server.shutdown().await.unwrap();
    must_succeed(robot.cancel_relay_reservation(reservation_a)).await;
    must_succeed(robot.cancel_relay_reservation(reservation_b)).await;
    must_succeed(compute.shutdown()).await;
    must_succeed(robot.shutdown()).await;
    relay_a.shutdown().await;
    relay_b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn relay_a_midstream_failure_restarts_from_zero_on_relay_b() {
    let dns = TestDns::start();
    let attempts = AttemptLog::default();
    let mut relay_a = RelayHarness::start("a-retry", true, attempts.clone()).await;
    let mut relay_b = RelayHarness::start("b-retry", true, attempts.clone()).await;
    let domain_id = Uuid::new_v4();

    let robot = node(&dns);
    let robot_credentials = P2pCredentialStore::new(robot.clone());
    install_current_token(&robot_credentials, &robot, PeerRole::Robot, domain_id).await;
    let mut incoming = robot
        .accept(
            ApplicationProtocol::new(DATASET_PROTOCOL).unwrap(),
            SessionRequirements::new(domain_id.to_string(), PeerRole::Compute).unwrap(),
        )
        .unwrap();

    let compute = node(&dns);
    let compute_credentials = P2pCredentialStore::new(compute.clone());
    let compute_token =
        install_current_token(&compute_credentials, &compute, PeerRole::Compute, domain_id).await;
    let compute_adapter = P2pDatasetAdapter::new(compute.clone(), compute_credentials, Vec::new());

    let provider_a = relay_a.provider();
    let provider_b = relay_b.provider();
    let reservation_a = must_succeed(robot.start_relay_reservation(provider_a)).await;
    let reservation_b = must_succeed(robot.start_relay_reservation(provider_b)).await;
    let route_a = must_succeed(robot.wait_relay_reservation(reservation_a))
        .await
        .publishable_route()
        .unwrap()
        .clone();
    let route_b = must_succeed(robot.wait_relay_reservation(reservation_b))
        .await
        .publishable_route()
        .unwrap()
        .clone();
    assert_eq!(
        timeout(relay_a.reservations.recv()).await,
        Some(robot.peer_id())
    );
    assert_eq!(
        timeout(relay_b.reservations.recv()).await,
        Some(robot.peer_id())
    );

    let bytes = zip_like_bytes(DATASET_BYTES);
    let sha256 = hex::encode(sha2::Sha256::digest(&bytes));
    let reference = P2pDatasetReference {
        schema: P2P_DATASET_SCHEMA.into(),
        dataset_id: "relay-midstream-retry".into(),
        domain_id,
        name: "relay-midstream-retry.zip".into(),
        peer_id: robot.peer_id().to_string(),
        multiaddrs: vec![route_a.to_string(), route_b.to_string()],
        size_bytes: bytes.len() as u64,
        sha256: sha256.clone(),
        available_until: Utc::now() + chrono::Duration::minutes(10),
    };

    let server_bytes = bytes.clone();
    let server = tokio::spawn(async move {
        for attempt in 0..2 {
            let mut stream = timeout(incoming.accept()).await.unwrap().unwrap();
            let request = read_dataset_request(&mut stream).await;
            assert_eq!(request.version, 0);
            assert_eq!(request.dataset_id, "relay-midstream-retry");
            write_dataset_header(
                &mut stream,
                &DatasetWireHeader {
                    dataset_id: request.dataset_id,
                    size_bytes: server_bytes.len() as u64,
                    sha256: sha256.clone(),
                },
            )
            .await;
            let body = if attempt == 0 {
                &server_bytes[..server_bytes.len() / 2]
            } else {
                server_bytes.as_slice()
            };
            stream.write_all(body).await.unwrap();
            stream.flush().await.unwrap();
            stream.close().await.unwrap();
        }
    });

    let relay_a_bytes_before = relay_a.transport_bytes();
    let relay_b_bytes_before = relay_b.transport_bytes();
    let temp = TempDir::new().unwrap();
    let destination = temp.path().join("relay-midstream-retry.zip");
    timeout(compute_adapter.fetch_dataset(&reference, &destination))
        .await
        .unwrap();
    timeout(server).await.unwrap();

    assert_eq!(fs::read(&destination).await.unwrap(), bytes);
    assert_no_partial_files(temp.path()).await;
    assert_eq!(
        attempts.lock().unwrap().as_slice(),
        ["a-retry", "b-retry"],
        "round two did not start at the sibling after relay A streamed and failed"
    );
    assert_admission(
        timeout(relay_a.admissions.recv()).await.unwrap(),
        compute.peer_id(),
        robot.peer_id(),
        domain_id,
        &compute_token,
    );
    assert_admission(
        timeout(relay_b.admissions.recv()).await.unwrap(),
        compute.peer_id(),
        robot.peer_id(),
        domain_id,
        &compute_token,
    );
    let circuit_a = timeout(relay_a.circuits.recv()).await.unwrap();
    assert_eq!(circuit_a.source_peer_id, compute.peer_id());
    assert_eq!(circuit_a.target_peer_id, robot.peer_id());
    let circuit_b = timeout(relay_b.circuits.recv()).await.unwrap();
    assert_eq!(circuit_b.source_peer_id, compute.peer_id());
    assert_eq!(circuit_b.target_peer_id, robot.peer_id());

    let relay_a_bytes = relay_a.transport_bytes() - relay_a_bytes_before;
    let relay_b_bytes = relay_b.transport_bytes() - relay_b_bytes_before;
    assert!(
        relay_a_bytes > (bytes.len() / 2) as u64,
        "relay A did not carry the truncated first body: {relay_a_bytes} bytes"
    );
    assert!(
        relay_b_bytes > bytes.len() as u64,
        "relay B did not carry the complete retried body: {relay_b_bytes} bytes"
    );
    assert_no_event(
        &mut relay_a.admissions,
        "unexpected third-round relay A admission",
    )
    .await;
    assert_no_event(
        &mut relay_b.admissions,
        "unexpected extra relay B admission",
    )
    .await;

    must_succeed(robot.cancel_relay_reservation(reservation_a)).await;
    must_succeed(robot.cancel_relay_reservation(reservation_b)).await;
    must_succeed(compute.shutdown()).await;
    must_succeed(robot.shutdown()).await;
    relay_a.shutdown().await;
    relay_b.shutdown().await;
}

fn publish_route(
    adapter: &P2pDatasetAdapter,
    index: u128,
    reservation: auki_p2p::RelayReservationHandle,
    route: Multiaddr,
) {
    adapter
        .publish_confirmed_relay_route(ConfirmedRelayRoute {
            fence: RelayRouteFence {
                slot_id: Uuid::from_u128(index),
                assignment_id: Uuid::from_u128(100 + index),
                reservation_epoch: Uuid::from_u128(200 + index),
                local_generation: reservation.generation().get(),
            },
            reservation,
            route,
            limits: relay_limits(),
            authorized_until: Utc::now() + chrono::Duration::minutes(10),
        })
        .unwrap();
}

fn route_from_provider(provider: &RelayProvider, target_peer_id: PeerId) -> Multiaddr {
    provider
        .reservation_listen_address()
        .with(Protocol::P2p(target_peer_id))
}

fn relay_limits() -> ExpectedRelayLimits {
    ExpectedRelayLimits::new(CIRCUIT_DURATION, CIRCUIT_DATA_BYTES).unwrap()
}

fn zip_like_bytes(length: usize) -> Vec<u8> {
    let mut bytes = vec![0_u8; length.max(4)];
    bytes[..4].copy_from_slice(b"PK\x03\x04");
    for (index, byte) in bytes[4..].iter_mut().enumerate() {
        *byte = (index % 251) as u8;
    }
    bytes
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DatasetWireRequest {
    version: u8,
    dataset_id: String,
}

#[derive(Serialize)]
struct DatasetWireHeader {
    dataset_id: String,
    size_bytes: u64,
    sha256: String,
}

async fn read_dataset_request(stream: &mut AuthenticatedStream) -> DatasetWireRequest {
    let mut length = [0_u8; 4];
    stream.read_exact(&mut length).await.unwrap();
    let length = u32::from_be_bytes(length) as usize;
    assert!((1..=4 * 1024).contains(&length));
    let mut encoded = vec![0_u8; length];
    stream.read_exact(&mut encoded).await.unwrap();
    serde_json::from_slice(&encoded).unwrap()
}

async fn write_dataset_header(stream: &mut AuthenticatedStream, header: &DatasetWireHeader) {
    let encoded = serde_json::to_vec(header).unwrap();
    assert!(encoded.len() <= 16 * 1024);
    stream
        .write_all(&(encoded.len() as u32).to_be_bytes())
        .await
        .unwrap();
    stream.write_all(&encoded).await.unwrap();
}

struct DirectFailureProbe {
    port: u16,
    connected: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    task: Option<thread::JoinHandle<()>>,
}

impl DirectFailureProbe {
    fn start(attempts: AttemptLog) -> Self {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let connected = Arc::new(AtomicBool::new(false));
        let task_connected = connected.clone();
        let shutdown = Arc::new(AtomicBool::new(false));
        let task_shutdown = shutdown.clone();
        let task = thread::spawn(move || {
            while !task_shutdown.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((_stream, _remote)) => {
                        if !task_connected.swap(true, Ordering::AcqRel) {
                            attempts.lock().unwrap().push("direct");
                        }
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("direct failure probe accept failed: {error}"),
                }
            }
        });
        Self {
            port,
            connected,
            shutdown,
            task: Some(task),
        }
    }

    fn route(&self) -> String {
        format!("/ip4/127.0.0.1/tcp/{}", self.port)
    }

    fn was_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }
}

impl Drop for DirectFailureProbe {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        if let Some(task) = self.task.take() {
            task.join().unwrap();
        }
    }
}

async fn assert_no_partial_files(directory: &Path) {
    let mut entries = fs::read_dir(directory).await.unwrap();
    while let Some(entry) = entries.next_entry().await.unwrap() {
        assert!(
            !entry.file_name().to_string_lossy().ends_with(".part"),
            "partial transfer file was not cleaned up"
        );
    }
}

async fn assert_no_event<T>(receiver: &mut mpsc::Receiver<T>, description: &str) {
    assert!(
        tokio::time::timeout(Duration::from_millis(100), receiver.recv())
            .await
            .is_err(),
        "{description}"
    );
}

fn node(dns: &TestDns) -> Node {
    let (resolver_config, resolver_options) = dns.resolver();
    Node::start_with_dns_config(
        Identity::generate(),
        verifier(),
        ["/ip4/127.0.0.1/tcp/0".parse().unwrap()],
        resolver_config,
        resolver_options,
    )
    .unwrap()
}

async fn install_current_token(
    credentials: &P2pCredentialStore,
    node: &Node,
    role: PeerRole,
    domain_id: Uuid,
) -> String {
    let issued_at = unix_time();
    let expires_at_unix = issued_at + P2P_TOKEN_TTL.as_secs();
    let claims = P2PAccessClaims {
        token_type: P2P_TOKEN_TYPE.into(),
        iss: P2P_TOKEN_ISSUER.into(),
        aud: vec![P2P_TOKEN_AUDIENCE.into()],
        sub: Uuid::new_v4().to_string(),
        peer_type: role,
        peer_id: node.peer_id().to_string(),
        domain_ids: vec![domain_id.to_string()],
        scopes: vec![P2P_TOKEN_SCOPE.into()],
        iat: issued_at,
        exp: expires_at_unix,
    };
    let token = encode(
        &Header::new(Algorithm::ES256),
        &claims,
        &EncodingKey::from_ec_pem(TEST_DDS_PRIVATE_KEY).unwrap(),
    )
    .unwrap();
    credentials
        .install(
            token.clone(),
            DateTime::from_timestamp(expires_at_unix as i64, 0).unwrap(),
        )
        .await
        .unwrap();
    token
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

async fn timeout<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(TEST_TIMEOUT, future)
        .await
        .expect("operation timed out")
}

async fn must_succeed<T, E: std::fmt::Debug>(
    future: impl std::future::Future<Output = Result<T, E>>,
) -> T {
    timeout(future).await.unwrap()
}

#[derive(NetworkBehaviour)]
struct RelayServerBehaviour {
    relay: relay::Behaviour,
    streams: StreamBehaviour,
}

struct RelayHarness {
    label: &'static str,
    peer_id: PeerId,
    port: u16,
    admissions: mpsc::Receiver<ObservedAdmission>,
    reservations: mpsc::Receiver<PeerId>,
    circuits: mpsc::Receiver<ObservedCircuit>,
    metrics: libp2p::metrics::Registry,
    swarm_task: JoinHandle<()>,
    admission_task: JoinHandle<()>,
}

impl RelayHarness {
    async fn start(label: &'static str, admission_allowed: bool, attempts: AttemptLog) -> Self {
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
            max_circuit_bytes: CIRCUIT_DATA_BYTES,
            circuit_src_rate_limiters: Vec::new(),
            ..Default::default()
        };
        let mut metrics = libp2p::metrics::Registry::default();
        let mut swarm = SwarmBuilder::with_existing_identity(identity)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )
            .unwrap()
            .with_bandwidth_metrics(&mut metrics)
            .with_behaviour(move |_| RelayServerBehaviour {
                relay: relay::Behaviour::new(peer_id, config),
                streams,
            })
            .unwrap()
            .build();
        swarm
            .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();
        let listen_address = loop {
            if let SwarmEvent::NewListenAddr { address, .. } = swarm.select_next_some().await {
                break address;
            }
        };
        swarm.add_external_address(listen_address.clone());
        let port = listen_address
            .iter()
            .find_map(|protocol| match protocol {
                Protocol::Tcp(port) => Some(port),
                _ => None,
            })
            .unwrap();

        let (reservation_sender, reservations) = mpsc::channel(8);
        let (circuit_sender, circuits) = mpsc::channel(8);
        let swarm_task = tokio::spawn(async move {
            while let Some(event) = swarm.next().await {
                if let SwarmEvent::Behaviour(RelayServerBehaviourEvent::Relay(event)) = event {
                    match event {
                        relay::Event::ReservationReqAccepted { src_peer_id, .. } => {
                            let _ = reservation_sender.send(src_peer_id).await;
                        }
                        relay::Event::CircuitReqAccepted {
                            src_peer_id,
                            dst_peer_id,
                        } => {
                            let _ = circuit_sender
                                .send(ObservedCircuit {
                                    source_peer_id: src_peer_id,
                                    target_peer_id: dst_peer_id,
                                })
                                .await;
                        }
                        _ => {}
                    }
                }
            }
        });
        let (admission_sender, admissions) = mpsc::channel(8);
        let allowed = Arc::new(AtomicBool::new(admission_allowed));
        let admission_task = tokio::spawn(serve_source_admission(
            incoming,
            admission_sender,
            allowed,
            label,
            attempts,
        ));
        Self {
            label,
            peer_id,
            port,
            admissions,
            reservations,
            circuits,
            metrics,
            swarm_task,
            admission_task,
        }
    }

    fn provider(&self) -> RelayProvider {
        RelayProvider::new(self.peer_id, [self.base()], relay_limits()).unwrap()
    }

    fn base(&self) -> String {
        format!(
            "/dns4/relay-{}.integration.auki-p2p.dev/tcp/{}/p2p/{}",
            self.label, self.port, self.peer_id
        )
    }

    fn transport_bytes(&self) -> u64 {
        let mut encoded = String::new();
        encode_metrics(&mut encoded, &self.metrics).unwrap();
        encoded
            .lines()
            .filter(|line| line.starts_with("libp2p_bandwidth_bytes_total{"))
            .map(|line| {
                line.rsplit_once(' ')
                    .unwrap_or_else(|| panic!("malformed bandwidth metric line: {line}"))
                    .1
                    .parse::<u64>()
                    .unwrap_or_else(|error| panic!("invalid bandwidth metric value: {error}"))
            })
            .sum()
    }

    async fn shutdown(self) {
        self.swarm_task.abort();
        self.admission_task.abort();
        let _ = self.swarm_task.await;
        let _ = self.admission_task.await;
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdmissionRequest {
    version: u8,
    domain_id: String,
    target_peer_id: String,
    p2p_access_token: String,
}

#[derive(Debug)]
struct ObservedAdmission {
    source_peer_id: PeerId,
    request: AdmissionRequest,
}

struct ObservedCircuit {
    source_peer_id: PeerId,
    target_peer_id: PeerId,
}

async fn serve_source_admission(
    mut incoming: IncomingStreams,
    admissions: mpsc::Sender<ObservedAdmission>,
    admission_allowed: Arc<AtomicBool>,
    relay_label: &'static str,
    attempts: AttemptLog,
) {
    while let Some((source_peer_id, mut stream)) = incoming.next().await {
        let mut length = [0; 4];
        stream.read_exact(&mut length).await.unwrap();
        let frame_size = u32::from_be_bytes(length) as usize;
        assert!((1..=64 * 1024).contains(&frame_size));
        let mut frame = vec![0; frame_size];
        stream.read_exact(&mut frame).await.unwrap();
        let request: AdmissionRequest = serde_json::from_slice(&frame).unwrap();
        attempts.lock().unwrap().push(relay_label);
        admissions
            .send(ObservedAdmission {
                source_peer_id,
                request,
            })
            .await
            .unwrap();

        let response = if admission_allowed.load(Ordering::Acquire) {
            serde_json::to_vec(&serde_json::json!({
                "accepted": true,
                "accepted_until": (Utc::now() + chrono::Duration::seconds(20))
                    .to_rfc3339_opts(SecondsFormat::Secs, true),
            }))
            .unwrap()
        } else {
            serde_json::to_vec(&serde_json::json!({ "accepted": false })).unwrap()
        };
        stream
            .write_all(&(response.len() as u32).to_be_bytes())
            .await
            .unwrap();
        stream.write_all(&response).await.unwrap();
        stream.flush().await.unwrap();
    }
}

fn assert_admission(
    admission: ObservedAdmission,
    source_peer_id: PeerId,
    target_peer_id: PeerId,
    domain_id: Uuid,
    token: &str,
) {
    assert_eq!(admission.source_peer_id, source_peer_id);
    assert_eq!(admission.request.version, 1);
    assert_eq!(admission.request.domain_id, domain_id.to_string());
    assert_eq!(admission.request.target_peer_id, target_peer_id.to_string());
    assert_eq!(admission.request.p2p_access_token, token);
}

struct TestDns {
    address: SocketAddr,
    shutdown: Arc<AtomicBool>,
    task: Option<thread::JoinHandle<()>>,
}

impl TestDns {
    fn start() -> Self {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let address = socket.local_addr().unwrap();
        let shutdown = Arc::new(AtomicBool::new(false));
        let task_shutdown = shutdown.clone();
        let task = thread::spawn(move || {
            let mut query = [0; 512];
            while !task_shutdown.load(Ordering::Acquire) {
                match socket.recv_from(&mut query) {
                    Ok((length, remote)) => {
                        if let Some(response) = dns_a_response(&query[..length]) {
                            socket.send_to(&response, remote).unwrap();
                        }
                    }
                    Err(error)
                        if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                    Err(error) => panic!("test DNS receive failed: {error}"),
                }
            }
        });
        Self {
            address,
            shutdown,
            task: Some(task),
        }
    }

    fn resolver(&self) -> (ResolverConfig, ResolverOpts) {
        let name_server = NameServerConfig::new(self.address, DnsProtocol::Udp);
        let config = ResolverConfig::from_parts(None, Vec::new(), vec![name_server]);
        let mut options = ResolverOpts::default();
        options.timeout = Duration::from_secs(1);
        options.attempts = 1;
        (config, options)
    }
}

impl Drop for TestDns {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        if let Some(task) = self.task.take() {
            task.join().unwrap();
        }
    }
}

fn dns_a_response(query: &[u8]) -> Option<Vec<u8>> {
    if query.len() < 17 || u16::from_be_bytes([query[4], query[5]]) != 1 {
        return None;
    }
    let mut cursor = 12;
    loop {
        let label_length = *query.get(cursor)? as usize;
        cursor += 1;
        if label_length == 0 {
            break;
        }
        cursor = cursor.checked_add(label_length)?;
        if cursor > query.len() {
            return None;
        }
    }
    let question_end = cursor.checked_add(4)?;
    let question = query.get(12..question_end)?;
    let query_type = u16::from_be_bytes([query[cursor], query[cursor + 1]]);

    let mut response = Vec::with_capacity(question_end + 16);
    response.extend_from_slice(&query[..2]);
    response.extend_from_slice(&0x8180u16.to_be_bytes());
    response.extend_from_slice(&1u16.to_be_bytes());
    response.extend_from_slice(&u16::from(query_type == 1).to_be_bytes());
    response.extend_from_slice(&0u16.to_be_bytes());
    response.extend_from_slice(&0u16.to_be_bytes());
    response.extend_from_slice(question);
    if query_type == 1 {
        response.extend_from_slice(&0xc00cu16.to_be_bytes());
        response.extend_from_slice(&1u16.to_be_bytes());
        response.extend_from_slice(&1u16.to_be_bytes());
        response.extend_from_slice(&60u32.to_be_bytes());
        response.extend_from_slice(&4u16.to_be_bytes());
        response.extend_from_slice(&Ipv4Addr::LOCALHOST.octets());
    }
    Some(response)
}

use std::{
    collections::HashSet,
    io::ErrorKind,
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use auki_p2p::{
    ApplicationProtocol, DdsTokenVerifier, ExpectedRelayLimits, Identity, Node, P2PAccessClaims,
    PeerRole, RelayProvider, RelayReservationState, SessionRequirements, P2P_TOKEN_AUDIENCE,
    P2P_TOKEN_ISSUER, P2P_TOKEN_SCOPE, P2P_TOKEN_TTL, P2P_TOKEN_TYPE,
};
use chrono::{SecondsFormat, Utc};
use futures::{
    io::{AsyncReadExt, AsyncWriteExt},
    StreamExt,
};
use hickory_resolver::{
    config::{NameServerConfig, ResolverConfig, ResolverOpts},
    proto::xfer::Protocol as DnsProtocol,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use libp2p::{
    multiaddr::Protocol,
    noise, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
};
use libp2p_stream::{Behaviour as StreamBehaviour, IncomingStreams};
use serde::Deserialize;
use tokio::{sync::mpsc, task::JoinHandle};
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

const APPLICATION_PROTOCOL: &str = "/auki-p2p/relay-test/1";
const SOURCE_ADMISSION_PROTOCOL: &str = "/auki-p2p/relay-auth/1";
const CIRCUIT_DURATION: Duration = Duration::from_secs(90);
const CIRCUIT_DATA_BYTES: u64 = 1_048_576;
const TEST_TIMEOUT: Duration = Duration::from_secs(15);

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn three_real_relays_confirm_route_exactly_and_cancel_generation_safely() {
    let dns = TestDns::start();
    let mut relay_a = RelayHarness::start("a").await;
    let mut relay_b = RelayHarness::start("b").await;
    let mut relay_c = RelayHarness::start("c").await;
    let domain_id = Uuid::new_v4().to_string();

    let target = node(&dns);
    install_current_token(&target, PeerRole::Robot, vec![domain_id.clone()]).await;
    let source = node(&dns);
    let source_token =
        install_current_token(&source, PeerRole::Compute, vec![domain_id.clone()]).await;

    let provider_a = relay_a.provider_with_fallback();
    let provider_b = relay_b.provider();
    let provider_c = relay_c.provider();
    let selected_a = provider_a.selected_base().clone();
    let mut relay_events = target.subscribe_relay_events();

    let reservation_a = must_succeed(target.start_relay_reservation(provider_a.clone())).await;
    let reservation_b = must_succeed(target.start_relay_reservation(provider_b.clone())).await;
    let reservation_c = must_succeed(target.start_relay_reservation(provider_c.clone())).await;
    let snapshot_a = must_succeed(target.wait_relay_reservation(reservation_a)).await;
    let snapshot_b = must_succeed(target.wait_relay_reservation(reservation_b)).await;
    let snapshot_c = must_succeed(target.wait_relay_reservation(reservation_c)).await;

    assert_eq!(snapshot_a.state(), RelayReservationState::Publishable);
    assert_eq!(snapshot_b.state(), RelayReservationState::Publishable);
    assert_eq!(snapshot_c.state(), RelayReservationState::Publishable);
    assert_eq!(snapshot_a.selected_base(), &selected_a);
    assert_eq!(snapshot_a.expected_limits(), relay_limits());
    assert_eq!(snapshot_b.expected_limits(), relay_limits());
    assert_eq!(snapshot_c.expected_limits(), relay_limits());
    let route_a = snapshot_a.publishable_route().unwrap().clone();
    let route_b = snapshot_b.publishable_route().unwrap().clone();
    let route_c = snapshot_c.publishable_route().unwrap().clone();
    assert_eq!(route_a, route_from_provider(&provider_a, target.peer_id()));
    assert_eq!(route_b, route_from_provider(&provider_b, target.peer_id()));
    assert_eq!(route_c, route_from_provider(&provider_c, target.peer_id()));

    // RESERVE is deliberately unauthenticated at the Auki layer. Only source
    // CONNECT preflight below may open relay-auth.
    assert!(relay_a.admissions.try_recv().is_err());
    assert!(relay_b.admissions.try_recv().is_err());
    assert!(relay_c.admissions.try_recv().is_err());

    wait_for_renewals(
        &mut relay_events,
        [reservation_a, reservation_b, reservation_c],
    )
    .await;

    let protocol = ApplicationProtocol::new(APPLICATION_PROTOCOL).unwrap();
    let mut incoming = target
        .accept(
            protocol.clone(),
            SessionRequirements::new(&domain_id, PeerRole::Compute).unwrap(),
        )
        .unwrap();
    let target_peer_id = target.peer_id();
    let source_peer_id = source.peer_id();
    let application_server = tokio::spawn(async move {
        let mut expected = [b'D', b'A', b'B', b'C', b'E'].into_iter();
        let mut observed_role_denial = false;
        while expected.len() != 0 || !observed_role_denial {
            match incoming.accept().await.unwrap() {
                Ok(mut stream) => {
                    assert_eq!(stream.remote_peer().peer_id, source_peer_id);
                    let mut request = [0; 1];
                    stream.read_exact(&mut request).await.unwrap();
                    let expected = expected.next().expect("unexpected application stream");
                    assert_eq!(request[0], expected);
                    stream.write_all(&request).await.unwrap();
                    stream.flush().await.unwrap();
                }
                Err(_) => observed_role_denial = true,
            }
        }
    });

    exchange_direct(&source, &target, &domain_id, protocol.clone(), b'D').await;

    exchange_over_route(
        &source,
        &route_a,
        &domain_id,
        target_peer_id,
        protocol.clone(),
        b'A',
    )
    .await;
    let admission_a = timeout(relay_a.admissions.recv()).await.unwrap();
    assert_admission(
        admission_a,
        source_peer_id,
        target_peer_id,
        &domain_id,
        &source_token,
    );
    assert_circuit(
        timeout(relay_a.circuits.recv()).await.unwrap(),
        source_peer_id,
        target_peer_id,
    );

    exchange_over_route(
        &source,
        &route_b,
        &domain_id,
        target_peer_id,
        protocol.clone(),
        b'B',
    )
    .await;
    let admission_b = timeout(relay_b.admissions.recv()).await.unwrap();
    assert_admission(
        admission_b,
        source_peer_id,
        target_peer_id,
        &domain_id,
        &source_token,
    );
    assert_circuit(
        timeout(relay_b.circuits.recv()).await.unwrap(),
        source_peer_id,
        target_peer_id,
    );

    exchange_over_route(
        &source,
        &route_c,
        &domain_id,
        target_peer_id,
        protocol.clone(),
        b'C',
    )
    .await;
    let admission_c = timeout(relay_c.admissions.recv()).await.unwrap();
    assert_admission(
        admission_c,
        source_peer_id,
        target_peer_id,
        &domain_id,
        &source_token,
    );
    assert_circuit(
        timeout(relay_c.circuits.recv()).await.unwrap(),
        source_peer_id,
        target_peer_id,
    );

    let requirements = SessionRequirements::new(&domain_id, PeerRole::Robot)
        .unwrap()
        .with_expected_remote_peer_id(target_peer_id);
    relay_c.set_admission_allowed(false);
    assert!(matches!(
        source.connect_relayed(route_c.clone(), &requirements).await,
        Err(auki_p2p::Error::RelayAdmissionDenied)
    ));
    let denied = timeout(relay_c.admissions.recv()).await.unwrap();
    assert_admission(
        denied,
        source_peer_id,
        target_peer_id,
        &domain_id,
        &source_token,
    );
    relay_c.set_admission_allowed(true);

    let wrong_domain = Uuid::new_v4().to_string();
    let wrong_domain_requirements = SessionRequirements::new(&wrong_domain, PeerRole::Robot)
        .unwrap()
        .with_expected_remote_peer_id(target_peer_id);
    assert!(matches!(
        source
            .connect_relayed(route_c.clone(), &wrong_domain_requirements)
            .await,
        Err(auki_p2p::Error::RemoteDomainMismatch(_))
    ));
    assert!(
        tokio::time::timeout(Duration::from_millis(100), relay_c.admissions.recv())
            .await
            .is_err()
    );

    let wrong_target = PeerId::random();
    let wrong_target_route = provider_c
        .reservation_listen_address()
        .with(Protocol::P2p(wrong_target));
    assert!(matches!(
        source
            .connect_relayed(wrong_target_route, &requirements)
            .await,
        Err(auki_p2p::Error::UnexpectedRemotePeer { .. })
    ));

    // Admission is role-neutral and supports a bounded multi-Domain token.
    for role in [PeerRole::Robot, PeerRole::DomainServer] {
        let role_token = install_current_token(
            &source,
            role,
            vec![domain_id.clone(), Uuid::new_v4().to_string()],
        )
        .await;
        let route_handle =
            must_succeed(source.connect_relayed(route_c.clone(), &requirements)).await;
        let observed = timeout(relay_c.admissions.recv()).await.unwrap();
        assert_admission(
            observed,
            source_peer_id,
            target_peer_id,
            &domain_id,
            &role_token,
        );
        must_succeed(source.close_relay_route(&route_handle)).await;
    }
    let source_token =
        install_current_token(&source, PeerRole::Compute, vec![domain_id.clone()]).await;

    let role_denied_route =
        must_succeed(source.connect_relayed(route_c.clone(), &requirements)).await;
    let observed = timeout(relay_c.admissions.recv()).await.unwrap();
    assert_admission(
        observed,
        source_peer_id,
        target_peer_id,
        &domain_id,
        &source_token,
    );
    let wrong_role = SessionRequirements::new(&domain_id, PeerRole::DomainServer)
        .unwrap()
        .with_expected_remote_peer_id(target_peer_id);
    assert!(source
        .open_relayed(&role_denied_route, protocol.clone(), wrong_role)
        .await
        .is_err());
    must_succeed(source.close_relay_route(&role_denied_route)).await;

    must_succeed(target.cancel_relay_reservation(reservation_a)).await;
    assert_eq!(
        must_succeed(target.relay_reservation(reservation_b))
            .await
            .state(),
        RelayReservationState::Publishable
    );
    assert!(target.relay_reservation(reservation_a).await.is_err());
    assert!(source
        .connect_relayed(route_a, &requirements)
        .await
        .is_err());

    exchange_direct(&source, &target, &domain_id, protocol, b'E').await;
    must_succeed(application_server).await;

    let replacement_a = must_succeed(target.start_relay_reservation(provider_a)).await;
    assert!(replacement_a.generation() > reservation_a.generation());
    let replacement_snapshot = must_succeed(target.wait_relay_reservation(replacement_a)).await;
    assert_eq!(
        replacement_snapshot.state(),
        RelayReservationState::Publishable
    );
    must_succeed(target.cancel_relay_reservation(replacement_a)).await;
    must_succeed(target.cancel_relay_reservation(reservation_b)).await;
    must_succeed(target.cancel_relay_reservation(reservation_c)).await;

    must_succeed(source.shutdown()).await;
    must_succeed(target.shutdown()).await;
    relay_a.shutdown().await;
    relay_b.shutdown().await;
    relay_c.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancellation_barrier_closes_every_direct_relay_connection_before_recreate() {
    let dns = TestDns::start();
    let mut relay = RelayHarness::start("cancel-many").await;
    let domain_id = Uuid::new_v4().to_string();
    let target = node(&dns);
    install_current_token(&target, PeerRole::Robot, vec![domain_id.clone()]).await;
    let provider = relay.provider();

    let reservation = must_succeed(target.start_relay_reservation(provider.clone())).await;
    must_succeed(target.wait_relay_reservation(reservation)).await;

    // `Node::open` always dials a fresh exact connection. The relay does not
    // serve this application protocol, so negotiation fails while each direct
    // transport connection remains alive until the reservation teardown.
    let protocol = ApplicationProtocol::new("/auki-p2p/cancel-probe/1").unwrap();
    let requirements = SessionRequirements::new(&domain_id, PeerRole::Compute)
        .unwrap()
        .with_expected_remote_peer_id(relay.peer_id);
    for prefix in ["extra-one", "extra-two"] {
        let address = relay.base(prefix).parse().unwrap();
        assert!(timeout(target.open(
            relay.peer_id,
            vec![address],
            protocol.clone(),
            requirements.clone(),
        ))
        .await
        .is_err());
    }
    let live_connections = relay.wait_for_active_connections(target.peer_id(), 3).await;

    let mut events = target.subscribe_relay_events();
    let cancel_target = target.clone();
    let mut cancellation =
        tokio::spawn(async move { cancel_target.cancel_relay_reservation(reservation).await });
    loop {
        match timeout(events.recv()).await.unwrap() {
            auki_p2p::RelayTransportEvent::Unpublished { handle } if handle == reservation => {
                break;
            }
            _ => {}
        }
    }

    // The selected connection is closed first. The tombstone must still
    // reject a replacement while the two extra connections are draining.
    assert!(target
        .start_relay_reservation(provider.clone())
        .await
        .is_err());
    must_succeed(&mut cancellation).await.unwrap();
    relay
        .wait_for_connections_closed(target.peer_id(), live_connections)
        .await;

    let replacement = must_succeed(target.start_relay_reservation(provider)).await;
    assert!(replacement.generation() > reservation.generation());
    must_succeed(target.wait_relay_reservation(replacement)).await;
    must_succeed(target.cancel_relay_reservation(replacement)).await;

    must_succeed(target.shutdown()).await;
    relay.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancellation_barrier_fences_a_direct_dial_that_establishes_late() {
    let dns = TestDns::start();
    let mut relay = RelayHarness::start("cancel-late").await;
    let domain_id = Uuid::new_v4().to_string();
    let target = node(&dns);
    install_current_token(&target, PeerRole::Robot, vec![domain_id.clone()]).await;
    let provider = relay.provider();

    let reservation = must_succeed(target.start_relay_reservation(provider.clone())).await;
    must_succeed(target.wait_relay_reservation(reservation)).await;
    let initial_connection = relay.wait_for_active_connections(target.peer_id(), 1).await;

    dns.hold_queries();
    let late_target = target.clone();
    let late_peer_id = relay.peer_id;
    let late_address = relay.base("late").parse().unwrap();
    let late_protocol = ApplicationProtocol::new("/auki-p2p/cancel-late/1").unwrap();
    let late_requirements = SessionRequirements::new(&domain_id, PeerRole::Compute)
        .unwrap()
        .with_expected_remote_peer_id(relay.peer_id);
    let late_open = tokio::spawn(async move {
        late_target
            .open(
                late_peer_id,
                vec![late_address],
                late_protocol,
                late_requirements,
            )
            .await
    });
    dns.wait_for_held_query().await;

    let cancel_target = target.clone();
    let mut cancellation =
        tokio::spawn(async move { cancel_target.cancel_relay_reservation(reservation).await });
    let early = tokio::time::timeout(Duration::from_millis(250), &mut cancellation).await;
    dns.release_queries();
    assert!(
        early.is_err(),
        "cancellation completed while a direct relay dial was still pending"
    );

    must_succeed(&mut cancellation).await.unwrap();
    assert!(must_succeed(late_open).await.is_err());
    relay
        .wait_for_connections_closed(target.peer_id(), initial_connection)
        .await;

    let replacement = must_succeed(target.start_relay_reservation(provider)).await;
    assert!(replacement.generation() > reservation.generation());
    must_succeed(target.wait_relay_reservation(replacement)).await;
    must_succeed(target.cancel_relay_reservation(replacement)).await;

    must_succeed(target.shutdown()).await;
    relay.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn real_limit_mismatch_remains_typed_before_waiter_registration() {
    let dns = TestDns::start();
    let relay = RelayHarness::start("limit-mismatch").await;
    let target = node(&dns);
    let mut events = target.subscribe_relay_events();
    let wrong_limits = ExpectedRelayLimits::new(CIRCUIT_DURATION, CIRCUIT_DATA_BYTES + 1).unwrap();
    let wrong_provider =
        RelayProvider::new(relay.peer_id, [relay.base("relay")], wrong_limits).unwrap();

    let rejected = must_succeed(target.start_relay_reservation(wrong_provider)).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(matches!(
        target.wait_relay_reservation(rejected).await,
        Err(auki_p2p::Error::RelayConfirmationRejected(
            auki_p2p::RelayConfirmationRejection::LimitMismatch { .. }
        ))
    ));
    loop {
        if matches!(
            timeout(events.recv()).await.unwrap(),
            auki_p2p::RelayTransportEvent::Canceled { handle } if handle == rejected
        ) {
            break;
        }
    }

    let replacement = must_succeed(target.start_relay_reservation(relay.provider())).await;
    assert!(replacement.generation() > rejected.generation());
    must_succeed(target.wait_relay_reservation(replacement)).await;
    must_succeed(target.cancel_relay_reservation(replacement)).await;
    must_succeed(target.shutdown()).await;
    relay.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn production_dns_resolution_failure_is_typed() {
    let dns = TestDns::start();
    let target = node(&dns);
    let relay_peer_id = PeerId::random();
    let provider = RelayProvider::new(
        relay_peer_id,
        [format!(
            "/dns4/missing.relay.auki-p2p.dev/tcp/443/p2p/{relay_peer_id}"
        )],
        relay_limits(),
    )
    .unwrap();
    assert!(matches!(
        target.start_relay_reservation(provider).await,
        Err(auki_p2p::Error::Dns(_))
    ));
    must_succeed(target.shutdown()).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn selected_relay_base_must_match_the_first_direct_connection() {
    let dns = TestDns::start();
    let mut relay = RelayHarness::start("wrong-base").await;
    let domain_id = Uuid::new_v4().to_string();
    let target = node(&dns);
    install_current_token(&target, PeerRole::Robot, vec![domain_id.clone()]).await;

    let wrong_address = relay.base("z-relay").parse().unwrap();
    let requirements = SessionRequirements::new(&domain_id, PeerRole::Compute)
        .unwrap()
        .with_expected_remote_peer_id(relay.peer_id);
    let protocol = ApplicationProtocol::new("/auki-p2p/wrong-base-probe/1").unwrap();
    assert!(target
        .open(relay.peer_id, vec![wrong_address], protocol, requirements,)
        .await
        .is_err());
    relay.wait_for_active_connections(target.peer_id(), 1).await;

    assert!(matches!(
        target
            .start_relay_reservation(relay.provider_with_fallback())
            .await,
        Err(auki_p2p::Error::RelayDirectConnectionMismatch { .. })
    ));
    assert!(relay.admissions.try_recv().is_err());

    must_succeed(target.shutdown()).await;
    relay.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn terminal_connection_close_remains_typed_before_wait_registration() {
    let dns = TestDns::start();
    let relay = RelayHarness::start("closed-before-wait").await;
    let target = node(&dns);
    let mut events = target.subscribe_relay_events();
    let handle = must_succeed(target.start_relay_reservation(relay.provider())).await;

    relay.shutdown().await;
    loop {
        if matches!(
            timeout(events.recv()).await.unwrap(),
            auki_p2p::RelayTransportEvent::Canceled { handle: canceled }
                if canceled == handle
        ) {
            break;
        }
    }
    assert!(matches!(
        target.wait_relay_reservation(handle).await,
        Err(auki_p2p::Error::RelayReservationClosed(_))
    ));

    must_succeed(target.shutdown()).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancel_before_confirmation_fences_the_old_dispatch_before_recreate() {
    let dns = TestDns::start();
    let relay = RelayHarness::start("cancel-before-confirm").await;
    let target = node(&dns);
    let provider = relay.provider();
    let mut events = target.subscribe_relay_events();

    relay.hold_after_next_connection();
    let old = must_succeed(target.start_relay_reservation(provider.clone())).await;
    relay.wait_until_held().await;

    let cancel_target = target.clone();
    let mut cancellation =
        tokio::spawn(async move { cancel_target.cancel_relay_reservation(old).await });
    loop {
        match timeout(events.recv()).await.unwrap() {
            auki_p2p::RelayTransportEvent::Unpublished { handle } if handle == old => break,
            auki_p2p::RelayTransportEvent::Publishable(snapshot) if snapshot.handle() == old => {
                panic!("the held reservation became publishable before cancellation")
            }
            _ => {}
        }
    }
    must_succeed(&mut cancellation).await.unwrap();

    let replacement_target = target.clone();
    let mut replacement =
        tokio::spawn(async move { replacement_target.start_relay_reservation(provider).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut replacement)
            .await
            .is_err()
    );
    relay.release_connection_hold();
    let replacement = must_succeed(replacement).await.unwrap();
    assert!(replacement.generation() > old.generation());
    must_succeed(target.wait_relay_reservation(replacement)).await;
    must_succeed(target.cancel_relay_reservation(replacement)).await;

    must_succeed(target.shutdown()).await;
    relay.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_established_circuit_may_open_after_its_connect_admission_expires() {
    let dns = TestDns::start();
    let mut relay = RelayHarness::start("admission-drain").await;
    relay.set_admission_ttl(Duration::from_secs(2));
    let domain_id = Uuid::new_v4().to_string();
    let target = node(&dns);
    install_current_token(&target, PeerRole::Robot, vec![domain_id.clone()]).await;
    let source = node(&dns);
    install_current_token(&source, PeerRole::Compute, vec![domain_id.clone()]).await;

    let reservation = must_succeed(target.start_relay_reservation(relay.provider())).await;
    let snapshot = must_succeed(target.wait_relay_reservation(reservation)).await;
    let route = snapshot.publishable_route().unwrap().clone();
    let target_peer_id = target.peer_id();
    let requirements = SessionRequirements::new(&domain_id, PeerRole::Robot)
        .unwrap()
        .with_expected_remote_peer_id(target_peer_id);
    let route_handle = must_succeed(source.connect_relayed(route, &requirements)).await;
    let _ = timeout(relay.admissions.recv()).await.unwrap();
    let _ = timeout(relay.circuits.recv()).await.unwrap();

    let protocol = ApplicationProtocol::new("/auki-p2p/admission-drain/1").unwrap();
    let mut incoming = target
        .accept(
            protocol.clone(),
            SessionRequirements::new(&domain_id, PeerRole::Compute).unwrap(),
        )
        .unwrap();
    let server = tokio::spawn(async move {
        let mut stream = incoming.accept().await.unwrap().unwrap();
        let mut byte = [0; 1];
        stream.read_exact(&mut byte).await.unwrap();
        stream.write_all(&byte).await.unwrap();
        stream.flush().await.unwrap();
    });

    let wait = route_handle
        .admission_expires_at()
        .signed_duration_since(Utc::now())
        .to_std()
        .unwrap_or_default()
        + Duration::from_millis(100);
    tokio::time::sleep(wait).await;
    let mut stream = must_succeed(source.open_relayed(&route_handle, protocol, requirements)).await;
    stream.write_all(b"x").await.unwrap();
    stream.flush().await.unwrap();
    let mut response = [0; 1];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(response, *b"x");
    must_succeed(server).await;

    must_succeed(source.close_relay_route(&route_handle)).await;
    must_succeed(target.cancel_relay_reservation(reservation)).await;
    must_succeed(source.shutdown()).await;
    must_succeed(target.shutdown()).await;
    relay.shutdown().await;
}

async fn exchange_over_route(
    source: &Node,
    route: &Multiaddr,
    domain_id: &str,
    target_peer_id: PeerId,
    protocol: ApplicationProtocol,
    marker: u8,
) {
    let requirements = SessionRequirements::new(domain_id, PeerRole::Robot)
        .unwrap()
        .with_expected_remote_peer_id(target_peer_id);
    let route_handle = must_succeed(source.connect_relayed(route.clone(), &requirements)).await;
    assert_eq!(route_handle.relay_peer_id(), relay_peer_id(route));
    assert_eq!(route_handle.target_peer_id(), target_peer_id);
    assert_eq!(route_handle.route(), route);

    let mut stream = must_succeed(source.open_relayed(&route_handle, protocol, requirements)).await;
    assert_eq!(stream.remote_peer().peer_id, target_peer_id);
    stream.write_all(&[marker]).await.unwrap();
    stream.flush().await.unwrap();
    let mut response = [0; 1];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(response[0], marker);
}

async fn exchange_direct(
    source: &Node,
    target: &Node,
    domain_id: &str,
    protocol: ApplicationProtocol,
    marker: u8,
) {
    let target_peer_id = target.peer_id();
    let target_address = must_succeed(target.first_listen_address()).await;
    let requirements = SessionRequirements::new(domain_id, PeerRole::Robot)
        .unwrap()
        .with_expected_remote_peer_id(target_peer_id);
    let mut stream =
        must_succeed(source.open(target_peer_id, vec![target_address], protocol, requirements))
            .await;
    stream.write_all(&[marker]).await.unwrap();
    stream.flush().await.unwrap();
    let mut response = [0; 1];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(response[0], marker);
}

async fn wait_for_renewals(
    events: &mut tokio::sync::broadcast::Receiver<auki_p2p::RelayTransportEvent>,
    reservations: [auki_p2p::RelayReservationHandle; 3],
) {
    let expected: HashSet<_> = reservations.into_iter().collect();
    let mut first_batch = HashSet::new();
    let mut second_batch = HashSet::new();
    while second_batch != expected {
        if let auki_p2p::RelayTransportEvent::Renewed(snapshot) =
            timeout(events.recv()).await.unwrap()
        {
            if !first_batch.insert(snapshot.handle()) {
                second_batch.insert(snapshot.handle());
            }
        }
    }
}

fn assert_admission(
    admission: ObservedAdmission,
    source_peer_id: PeerId,
    target_peer_id: PeerId,
    domain_id: &str,
    token: &str,
) {
    assert_eq!(admission.source_peer_id, source_peer_id);
    assert_eq!(admission.request.version, 1);
    assert_eq!(admission.request.domain_id, domain_id);
    assert_eq!(admission.request.target_peer_id, target_peer_id.to_string());
    assert_eq!(admission.request.p2p_access_token, token);
}

fn assert_circuit(circuit: ObservedCircuit, source_peer_id: PeerId, target_peer_id: PeerId) {
    assert_eq!(circuit.source_peer_id, source_peer_id);
    assert_eq!(circuit.target_peer_id, target_peer_id);
}

fn route_from_provider(provider: &RelayProvider, target_peer_id: PeerId) -> Multiaddr {
    provider
        .reservation_listen_address()
        .with(Protocol::P2p(target_peer_id))
}

fn relay_peer_id(route: &Multiaddr) -> PeerId {
    route
        .iter()
        .find_map(|protocol| match protocol {
            Protocol::P2p(peer_id) => Some(peer_id),
            _ => None,
        })
        .unwrap()
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
    admission_allowed: Arc<AtomicBool>,
    admission_ttl_seconds: Arc<AtomicU64>,
    hold_after_connection: Arc<AtomicBool>,
    connection_held: Arc<AtomicBool>,
    circuits: mpsc::Receiver<ObservedCircuit>,
    connections: mpsc::Receiver<RelayConnectionEvent>,
    swarm_task: JoinHandle<()>,
    admission_task: JoinHandle<()>,
}

impl RelayHarness {
    async fn start(label: &'static str) -> Self {
        let identity = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = identity.public().to_peer_id();
        let streams = StreamBehaviour::new();
        let mut control = streams.new_control();
        let incoming = control
            .accept(StreamProtocol::new(SOURCE_ADMISSION_PROTOCOL))
            .unwrap();
        let config = relay::Config {
            reservation_duration: Duration::from_secs(4),
            reservation_rate_limiters: Vec::new(),
            max_circuit_duration: CIRCUIT_DURATION,
            max_circuit_bytes: CIRCUIT_DATA_BYTES,
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

        let (connection_sender, connections) = mpsc::channel(64);
        let (circuit_sender, circuits) = mpsc::channel(64);
        let hold_after_connection = Arc::new(AtomicBool::new(false));
        let task_hold_after_connection = hold_after_connection.clone();
        let connection_held = Arc::new(AtomicBool::new(false));
        let task_connection_held = connection_held.clone();
        let swarm_task = tokio::spawn(async move {
            while let Some(event) = swarm.next().await {
                if let SwarmEvent::Behaviour(RelayServerBehaviourEvent::Relay(
                    relay::Event::CircuitReqAccepted {
                        src_peer_id,
                        dst_peer_id,
                    },
                )) = &event
                {
                    let _ = circuit_sender
                        .send(ObservedCircuit {
                            source_peer_id: *src_peer_id,
                            target_peer_id: *dst_peer_id,
                        })
                        .await;
                }
                let observed = match event {
                    SwarmEvent::ConnectionEstablished {
                        peer_id,
                        connection_id,
                        ..
                    } => Some(RelayConnectionEvent::Established {
                        peer_id,
                        connection_id,
                    }),
                    SwarmEvent::ConnectionClosed {
                        peer_id,
                        connection_id,
                        ..
                    } => Some(RelayConnectionEvent::Closed {
                        peer_id,
                        connection_id,
                    }),
                    _ => None,
                };
                if let Some(observed) = observed {
                    let _ = connection_sender.send(observed).await;
                    if task_hold_after_connection.swap(false, Ordering::AcqRel) {
                        task_connection_held.store(true, Ordering::Release);
                        while task_connection_held.load(Ordering::Acquire) {
                            tokio::time::sleep(Duration::from_millis(5)).await;
                        }
                    }
                }
            }
        });
        let (admission_sender, admissions) = mpsc::channel(8);
        let admission_allowed = Arc::new(AtomicBool::new(true));
        let admission_ttl_seconds = Arc::new(AtomicU64::new(20));
        let admission_task = tokio::spawn(serve_source_admission(
            incoming,
            admission_sender,
            admission_allowed.clone(),
            admission_ttl_seconds.clone(),
        ));

        Self {
            label,
            peer_id,
            port,
            admissions,
            admission_allowed,
            admission_ttl_seconds,
            hold_after_connection,
            connection_held,
            circuits,
            connections,
            swarm_task,
            admission_task,
        }
    }

    fn provider(&self) -> RelayProvider {
        RelayProvider::new(self.peer_id, [self.base("relay")], relay_limits()).unwrap()
    }

    fn provider_with_fallback(&self) -> RelayProvider {
        RelayProvider::new(
            self.peer_id,
            [self.base("z-relay"), self.base("a-relay")],
            relay_limits(),
        )
        .unwrap()
    }

    fn base(&self, prefix: &str) -> String {
        format!(
            "/dns4/{prefix}-{}.relay.auki-p2p.dev/tcp/{}/p2p/{}",
            self.label, self.port, self.peer_id
        )
    }

    fn set_admission_allowed(&self, allowed: bool) {
        self.admission_allowed.store(allowed, Ordering::Release);
    }

    fn set_admission_ttl(&self, ttl: Duration) {
        self.admission_ttl_seconds
            .store(ttl.as_secs(), Ordering::Release);
    }

    fn hold_after_next_connection(&self) {
        self.connection_held.store(false, Ordering::Release);
        self.hold_after_connection.store(true, Ordering::Release);
    }

    async fn wait_until_held(&self) {
        timeout(async {
            while !self.connection_held.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await;
    }

    fn release_connection_hold(&self) {
        self.connection_held.store(false, Ordering::Release);
    }

    async fn wait_for_active_connections(
        &mut self,
        peer_id: PeerId,
        expected: usize,
    ) -> HashSet<libp2p::swarm::ConnectionId> {
        let mut active = HashSet::new();
        while active.len() < expected {
            match timeout(self.connections.recv()).await.unwrap() {
                RelayConnectionEvent::Established {
                    peer_id: observed,
                    connection_id,
                } if observed == peer_id => {
                    active.insert(connection_id);
                }
                RelayConnectionEvent::Closed {
                    peer_id: observed,
                    connection_id,
                } if observed == peer_id => {
                    active.remove(&connection_id);
                }
                _ => {}
            }
        }
        active
    }

    async fn wait_for_connections_closed(
        &mut self,
        peer_id: PeerId,
        mut active: HashSet<libp2p::swarm::ConnectionId>,
    ) {
        while !active.is_empty() {
            match timeout(self.connections.recv()).await.unwrap() {
                RelayConnectionEvent::Established {
                    peer_id: observed,
                    connection_id,
                } if observed == peer_id => {
                    active.insert(connection_id);
                }
                RelayConnectionEvent::Closed {
                    peer_id: observed,
                    connection_id,
                } if observed == peer_id => {
                    active.remove(&connection_id);
                }
                _ => {}
            }
        }
    }

    async fn shutdown(self) {
        self.connection_held.store(false, Ordering::Release);
        self.swarm_task.abort();
        self.admission_task.abort();
        let _ = self.swarm_task.await;
        let _ = self.admission_task.await;
    }
}

enum RelayConnectionEvent {
    Established {
        peer_id: PeerId,
        connection_id: libp2p::swarm::ConnectionId,
    },
    Closed {
        peer_id: PeerId,
        connection_id: libp2p::swarm::ConnectionId,
    },
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
    admission_ttl_seconds: Arc<AtomicU64>,
) {
    while let Some((source_peer_id, mut stream)) = incoming.next().await {
        let mut length = [0; 4];
        stream.read_exact(&mut length).await.unwrap();
        let frame_size = u32::from_be_bytes(length) as usize;
        assert!((1..=64 * 1024).contains(&frame_size));
        let mut frame = vec![0; frame_size];
        stream.read_exact(&mut frame).await.unwrap();
        let request: AdmissionRequest = serde_json::from_slice(&frame).unwrap();
        admissions
            .send(ObservedAdmission {
                source_peer_id,
                request,
            })
            .await
            .unwrap();

        let response = if admission_allowed.load(Ordering::Acquire) {
            let accepted_until = (Utc::now()
                + chrono::Duration::seconds(admission_ttl_seconds.load(Ordering::Acquire) as i64))
            .to_rfc3339_opts(SecondsFormat::Secs, true);
            serde_json::to_vec(&serde_json::json!({
                "accepted": true,
                "accepted_until": accepted_until,
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

fn relay_limits() -> ExpectedRelayLimits {
    ExpectedRelayLimits::new(CIRCUIT_DURATION, CIRCUIT_DATA_BYTES).unwrap()
}

struct TestDns {
    address: SocketAddr,
    shutdown: Arc<AtomicBool>,
    hold_queries: Arc<AtomicBool>,
    held_query_seen: Arc<AtomicBool>,
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
        let thread_shutdown = shutdown.clone();
        let hold_queries = Arc::new(AtomicBool::new(false));
        let thread_hold_queries = hold_queries.clone();
        let held_query_seen = Arc::new(AtomicBool::new(false));
        let thread_held_query_seen = held_query_seen.clone();
        let task = thread::spawn(move || {
            let mut query = [0; 512];
            while !thread_shutdown.load(Ordering::Acquire) {
                match socket.recv_from(&mut query) {
                    Ok((length, remote)) => {
                        if thread_hold_queries.load(Ordering::Acquire) {
                            thread_held_query_seen.store(true, Ordering::Release);
                            while thread_hold_queries.load(Ordering::Acquire)
                                && !thread_shutdown.load(Ordering::Acquire)
                            {
                                thread::sleep(Duration::from_millis(5));
                            }
                        }
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
            hold_queries,
            held_query_seen,
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

    fn hold_queries(&self) {
        self.held_query_seen.store(false, Ordering::Release);
        self.hold_queries.store(true, Ordering::Release);
    }

    async fn wait_for_held_query(&self) {
        timeout(async {
            while !self.held_query_seen.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await;
    }

    fn release_queries(&self) {
        self.hold_queries.store(false, Ordering::Release);
    }
}

impl Drop for TestDns {
    fn drop(&mut self) {
        self.hold_queries.store(false, Ordering::Release);
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
    let missing = query[12..cursor]
        .windows(b"missing".len())
        .any(|window| window == b"missing");

    let mut response = Vec::with_capacity(question_end + 16);
    response.extend_from_slice(&query[..2]);
    response.extend_from_slice(&(if missing { 0x8183u16 } else { 0x8180u16 }).to_be_bytes());
    response.extend_from_slice(&1u16.to_be_bytes());
    response.extend_from_slice(&u16::from(query_type == 1 && !missing).to_be_bytes());
    response.extend_from_slice(&0u16.to_be_bytes());
    response.extend_from_slice(&0u16.to_be_bytes());
    response.extend_from_slice(question);
    if query_type == 1 && !missing {
        response.extend_from_slice(&0xc00cu16.to_be_bytes());
        response.extend_from_slice(&1u16.to_be_bytes());
        response.extend_from_slice(&1u16.to_be_bytes());
        response.extend_from_slice(&60u32.to_be_bytes());
        response.extend_from_slice(&4u16.to_be_bytes());
        response.extend_from_slice(&Ipv4Addr::LOCALHOST.octets());
    }
    Some(response)
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

async fn install_current_token(node: &Node, role: PeerRole, domain_ids: Vec<String>) -> String {
    let claims = P2PAccessClaims {
        token_type: P2P_TOKEN_TYPE.into(),
        iss: P2P_TOKEN_ISSUER.into(),
        aud: vec![P2P_TOKEN_AUDIENCE.into()],
        sub: Uuid::new_v4().to_string(),
        peer_type: role,
        peer_id: node.peer_id().to_string(),
        domain_ids,
        scopes: vec![P2P_TOKEN_SCOPE.into()],
        iat: unix_time(),
        exp: unix_time() + P2P_TOKEN_TTL.as_secs(),
    };
    let token = encode(
        &Header::new(Algorithm::ES256),
        &claims,
        &EncodingKey::from_ec_pem(TEST_DDS_PRIVATE_KEY).unwrap(),
    )
    .unwrap();
    node.install_token(token.clone()).await.unwrap();
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

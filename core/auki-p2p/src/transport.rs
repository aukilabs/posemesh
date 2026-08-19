use std::{
    collections::HashMap,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    StreamExt,
};
use libp2p::{
    multiaddr::Protocol,
    noise,
    swarm::{dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Stream, StreamProtocol, Swarm, SwarmBuilder,
};
use libp2p_stream::{Behaviour as StreamBehaviour, IncomingStreams};
use tokio::sync::{mpsc, oneshot, watch};
use uuid::Uuid;

use crate::{
    token::{ensure_token_peer, DdsTokenVerifier, P2PAccessClaims, PeerRole, TokenStore},
    Error, Identity, Result as P2PResult,
};

const MAX_TOKEN_BYTES: usize = 64 * 1024;
const AUTHENTICATION_TIMEOUT: Duration = Duration::from_secs(10);
const AUTH_ACCEPTED: u8 = 1;
const AUTH_REJECTED: u8 = 0;

#[derive(NetworkBehaviour)]
struct Behaviour {
    streams: StreamBehaviour,
}

#[derive(Clone, Debug)]
pub struct ApplicationProtocol(StreamProtocol);

impl ApplicationProtocol {
    pub fn new(value: impl Into<String>) -> P2PResult<Self> {
        let value = value.into();
        let components: Vec<_> = value.split('/').collect();
        if components.len() < 4
            || !components[0].is_empty()
            || components[1] != "auki-p2p"
            || components[2].is_empty()
            || !components[3..].iter().any(|component| {
                component
                    .chars()
                    .any(|character| character.is_ascii_digit())
            })
        {
            return Err(Error::InvalidProtocol(
                "expected /auki-p2p/<application>/<version>".into(),
            ));
        }
        let protocol = StreamProtocol::try_from_owned(value)
            .map_err(|error| Error::InvalidProtocol(error.to_string()))?;
        Ok(Self(protocol))
    }
}

#[derive(Clone, Debug)]
pub struct SessionRequirements {
    domain_id: Uuid,
    remote_role: PeerRole,
    expected_remote_peer_id: Option<PeerId>,
}

impl SessionRequirements {
    pub fn new(domain_id: impl Into<String>, remote_role: PeerRole) -> P2PResult<Self> {
        let domain_id = domain_id.into();
        let domain_id = Uuid::parse_str(&domain_id)
            .map_err(|_| Error::InvalidToken("required Domain must be a UUID".into()))?;
        Ok(Self {
            domain_id,
            remote_role,
            expected_remote_peer_id: None,
        })
    }

    pub fn with_expected_remote_peer_id(mut self, peer_id: PeerId) -> Self {
        self.expected_remote_peer_id = Some(peer_id);
        self
    }

    fn validate(&self, claims: &P2PAccessClaims, noise_peer_id: PeerId) -> P2PResult<()> {
        if let Some(expected) = self.expected_remote_peer_id {
            if expected != noise_peer_id {
                return Err(Error::UnexpectedRemotePeer {
                    expected: expected.to_string(),
                    actual: noise_peer_id.to_string(),
                });
            }
        }
        if claims.peer_type != self.remote_role {
            return Err(Error::RemoteRoleMismatch {
                expected: self.remote_role.to_string(),
                actual: claims.peer_type.to_string(),
            });
        }
        if !claims
            .domain_ids
            .iter()
            .filter_map(|domain_id| Uuid::parse_str(domain_id).ok())
            .any(|domain_id| domain_id == self.domain_id)
        {
            return Err(Error::RemoteDomainMismatch(self.domain_id.to_string()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct AuthenticatedPeer {
    pub peer_id: PeerId,
    pub subject: String,
    pub role: PeerRole,
    pub domain_ids: Vec<String>,
}

/// The public byte-stream boundary. The inner libp2p stream is deliberately not
/// exposed and this wrapper can only be constructed after mutual DDS auth.
pub struct AuthenticatedStream {
    inner: Stream,
    remote: AuthenticatedPeer,
}

impl std::fmt::Debug for AuthenticatedStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthenticatedStream")
            .field("remote", &self.remote)
            .finish_non_exhaustive()
    }
}

impl AuthenticatedStream {
    pub fn remote_peer(&self) -> &AuthenticatedPeer {
        &self.remote
    }
}

impl AsyncRead for AuthenticatedStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_read(context, buffer)
    }
}

impl AsyncWrite for AuthenticatedStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(context, buffer)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(context)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_close(context)
    }
}

pub struct IncomingAuthenticatedStreams {
    inner: IncomingStreams,
    local_peer_id: PeerId,
    tokens: TokenStore,
    verifier: DdsTokenVerifier,
    requirements: SessionRequirements,
}

impl IncomingAuthenticatedStreams {
    pub async fn accept(&mut self) -> Option<P2PResult<AuthenticatedStream>> {
        let (remote_peer_id, stream) = self.inner.next().await?;
        Some(
            authenticate(
                stream,
                self.local_peer_id,
                remote_peer_id,
                &self.tokens,
                &self.verifier,
                &self.requirements,
            )
            .await,
        )
    }
}

#[derive(Clone)]
pub struct Node {
    identity: Identity,
    control: libp2p_stream::Control,
    tokens: TokenStore,
    verifier: DdsTokenVerifier,
    command_sender: mpsc::Sender<Command>,
    listen_addresses: watch::Receiver<Vec<Multiaddr>>,
}

impl Node {
    pub fn start(
        identity: Identity,
        verifier: DdsTokenVerifier,
        listen_addresses: impl IntoIterator<Item = Multiaddr>,
    ) -> P2PResult<Self> {
        let stream_behaviour = StreamBehaviour::new();
        let control = stream_behaviour.new_control();
        let behaviour = Behaviour {
            streams: stream_behaviour,
        };
        let mut swarm = build_swarm(identity.keypair(), behaviour)?;
        for address in listen_addresses {
            swarm
                .listen_on(address.clone())
                .map_err(|error| Error::Listen {
                    address: address.to_string(),
                    reason: error.to_string(),
                })?;
        }

        let (command_sender, command_receiver) = mpsc::channel(16);
        let (listen_sender, listen_receiver) = watch::channel(Vec::new());
        let runtime =
            tokio::runtime::Handle::try_current().map_err(|_| Error::RuntimeUnavailable)?;
        runtime.spawn(run_swarm(swarm, command_receiver, listen_sender));

        Ok(Self {
            identity,
            control,
            tokens: TokenStore::default(),
            verifier,
            command_sender,
            listen_addresses: listen_receiver,
        })
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub fn peer_id(&self) -> PeerId {
        self.identity.peer_id()
    }

    pub async fn install_token(&self, token: impl Into<String>) -> P2PResult<P2PAccessClaims> {
        self.tokens
            .install(token.into(), &self.verifier, self.peer_id())
            .await
    }

    pub async fn clear_token(&self) {
        self.tokens.clear().await;
    }

    pub async fn first_listen_address(&self) -> P2PResult<Multiaddr> {
        let mut receiver = self.listen_addresses.clone();
        loop {
            if let Some(address) = receiver.borrow().first().cloned() {
                return Ok(address);
            }
            receiver.changed().await.map_err(|_| Error::SwarmStopped)?;
        }
    }

    pub fn accept(
        &self,
        protocol: ApplicationProtocol,
        requirements: SessionRequirements,
    ) -> P2PResult<IncomingAuthenticatedStreams> {
        let mut control = self.control.clone();
        let inner = control
            .accept(protocol.0)
            .map_err(|_| Error::ProtocolAlreadyRegistered)?;
        Ok(IncomingAuthenticatedStreams {
            inner,
            local_peer_id: self.peer_id(),
            tokens: self.tokens.clone(),
            verifier: self.verifier.clone(),
            requirements,
        })
    }

    pub async fn open(
        &self,
        remote_peer_id: PeerId,
        remote_addresses: Vec<Multiaddr>,
        protocol: ApplicationProtocol,
        requirements: SessionRequirements,
    ) -> P2PResult<AuthenticatedStream> {
        if remote_addresses.is_empty() {
            return Err(Error::MissingRemoteAddress);
        }
        if let Some(expected) = requirements.expected_remote_peer_id {
            if expected != remote_peer_id {
                return Err(Error::UnexpectedRemotePeer {
                    expected: expected.to_string(),
                    actual: remote_peer_id.to_string(),
                });
            }
        }

        let addresses = remote_addresses
            .into_iter()
            .map(|address| normalize_remote_address(address, remote_peer_id))
            .collect::<P2PResult<Vec<_>>>()?;
        self.send_command(|response| Command::Connect {
            peer_id: remote_peer_id,
            addresses,
            response,
        })
        .await?;

        let mut control = self.control.clone();
        let stream = control
            .open_stream(remote_peer_id, protocol.0)
            .await
            .map_err(|error| Error::OpenStream(error.to_string()))?;
        authenticate(
            stream,
            self.peer_id(),
            remote_peer_id,
            &self.tokens,
            &self.verifier,
            &requirements,
        )
        .await
    }

    pub async fn disconnect(&self, peer_id: PeerId) -> P2PResult<()> {
        self.send_command(|response| Command::Disconnect { peer_id, response })
            .await
    }

    pub async fn shutdown(&self) -> P2PResult<()> {
        self.send_command(|response| Command::Shutdown { response })
            .await
    }

    async fn send_command(
        &self,
        build: impl FnOnce(oneshot::Sender<P2PResult<()>>) -> Command,
    ) -> P2PResult<()> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.command_sender
            .send(build(response_sender))
            .await
            .map_err(|_| Error::SwarmStopped)?;
        response_receiver.await.map_err(|_| Error::SwarmStopped)?
    }
}

enum Command {
    Connect {
        peer_id: PeerId,
        addresses: Vec<Multiaddr>,
        response: oneshot::Sender<P2PResult<()>>,
    },
    Disconnect {
        peer_id: PeerId,
        response: oneshot::Sender<P2PResult<()>>,
    },
    Shutdown {
        response: oneshot::Sender<P2PResult<()>>,
    },
}

fn build_swarm(
    identity: libp2p::identity::Keypair,
    behaviour: Behaviour,
) -> P2PResult<Swarm<Behaviour>> {
    SwarmBuilder::with_existing_identity(identity)
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true),
            noise::Config::new,
            yamux::Config::default,
        )
        .map_err(|error| Error::TransportBuild(error.to_string()))?
        .with_behaviour(|_| behaviour)
        .map_err(|error| Error::TransportBuild(error.to_string()))
        .map(|builder| {
            builder
                .with_swarm_config(|config| {
                    config.with_idle_connection_timeout(Duration::from_secs(60))
                })
                .build()
        })
}

async fn run_swarm(
    mut swarm: Swarm<Behaviour>,
    mut commands: mpsc::Receiver<Command>,
    listen_addresses: watch::Sender<Vec<Multiaddr>>,
) {
    let mut pending_dials: HashMap<PeerId, Vec<oneshot::Sender<P2PResult<()>>>> = HashMap::new();
    let mut pending_disconnects: HashMap<PeerId, Vec<oneshot::Sender<P2PResult<()>>>> =
        HashMap::new();
    loop {
        tokio::select! {
            event = swarm.next() => {
                let Some(event) = event else { break };
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        listen_addresses.send_modify(|addresses| {
                            if !addresses.contains(&address) {
                                addresses.push(address);
                            }
                        });
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        if let Some(responses) = pending_dials.remove(&peer_id) {
                            for response in responses {
                                let _ = response.send(Ok(()));
                            }
                        }
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id: Some(peer_id), error, .. } => {
                        if let Some(responses) = pending_dials.remove(&peer_id) {
                            let reason = error.to_string();
                            for response in responses {
                                let _ = response.send(Err(Error::Dial(reason.clone())));
                            }
                        }
                    }
                    SwarmEvent::ConnectionClosed { peer_id, num_established: 0, .. } => {
                        if let Some(responses) = pending_disconnects.remove(&peer_id) {
                            for response in responses {
                                let _ = response.send(Ok(()));
                            }
                        }
                    }
                    _ => {}
                }
            }
            command = commands.recv() => {
                let Some(command) = command else { break };
                match command {
                    Command::Connect { peer_id, addresses, response } => {
                        if swarm.is_connected(&peer_id) {
                            let _ = response.send(Ok(()));
                            continue;
                        }
                        if let Some(responses) = pending_dials.get_mut(&peer_id) {
                            responses.push(response);
                            continue;
                        }
                        match swarm.dial(
                            DialOpts::peer_id(peer_id)
                                .allocate_new_port()
                                .addresses(addresses)
                                .build(),
                        ) {
                            Ok(()) => {
                                pending_dials.insert(peer_id, vec![response]);
                            }
                            Err(error) => {
                                let _ = response.send(Err(Error::Dial(error.to_string())));
                            }
                        }
                    }
                    Command::Disconnect { peer_id, response } => {
                        if let Some(responses) = pending_disconnects.get_mut(&peer_id) {
                            responses.push(response);
                            continue;
                        }
                        match swarm.disconnect_peer_id(peer_id) {
                            Ok(()) => {
                                pending_disconnects.insert(peer_id, vec![response]);
                            }
                            Err(()) => {
                                let _ = response.send(Err(Error::Disconnect(peer_id.to_string())));
                            }
                        }
                    }
                    Command::Shutdown { response } => {
                        let _ = response.send(Ok(()));
                        break;
                    }
                }
            }
        }
    }
}

fn normalize_remote_address(mut address: Multiaddr, peer_id: PeerId) -> P2PResult<Multiaddr> {
    if let Some(Protocol::P2p(address_peer_id)) = address.iter().last() {
        if address_peer_id != peer_id {
            return Err(Error::InvalidRemoteAddress {
                address: address.to_string(),
                reason: format!("contains different Peer ID {address_peer_id}"),
            });
        }
        address.pop();
    }
    let has_tcp = address
        .iter()
        .any(|protocol| matches!(protocol, Protocol::Tcp(_)));
    if !has_tcp {
        return Err(Error::InvalidRemoteAddress {
            address: address.to_string(),
            reason: "only explicit TCP multiaddrs are supported".into(),
        });
    }
    Ok(address)
}

async fn authenticate(
    stream: Stream,
    local_peer_id: PeerId,
    remote_peer_id: PeerId,
    tokens: &TokenStore,
    verifier: &DdsTokenVerifier,
    requirements: &SessionRequirements,
) -> P2PResult<AuthenticatedStream> {
    tokio::time::timeout(
        AUTHENTICATION_TIMEOUT,
        authenticate_inner(
            stream,
            local_peer_id,
            remote_peer_id,
            tokens,
            verifier,
            requirements,
        ),
    )
    .await
    .map_err(|_| Error::AuthenticationTimeout)?
}

async fn authenticate_inner(
    mut stream: Stream,
    local_peer_id: PeerId,
    remote_peer_id: PeerId,
    tokens: &TokenStore,
    verifier: &DdsTokenVerifier,
    requirements: &SessionRequirements,
) -> P2PResult<AuthenticatedStream> {
    let local_token = tokens.snapshot().await.unwrap_or_default();
    write_token_frame(&mut stream, local_token.as_bytes()).await?;
    let remote_token = read_token_frame(&mut stream).await;

    let local_result = if local_token.is_empty() {
        Err(Error::MissingToken)
    } else {
        verifier
            .verify(&local_token)
            .and_then(|claims| ensure_token_peer(&claims, local_peer_id).map(|_| claims))
    };
    let remote_result = remote_token.and_then(|token| {
        let token = String::from_utf8(token).map_err(|_| Error::InvalidTokenEncoding)?;
        let claims = verifier.verify(&token)?;
        ensure_token_peer(&claims, remote_peer_id)?;
        requirements.validate(&claims, remote_peer_id)?;
        Ok(claims)
    });

    let local_accepts = local_result.is_ok() && remote_result.is_ok();
    stream
        .write_all(&[if local_accepts {
            AUTH_ACCEPTED
        } else {
            AUTH_REJECTED
        }])
        .await?;
    stream.flush().await?;

    let mut remote_status = [AUTH_REJECTED];
    stream.read_exact(&mut remote_status).await?;
    local_result?;
    let remote_claims = remote_result?;
    if remote_status[0] != AUTH_ACCEPTED {
        return Err(Error::RemoteRejected);
    }

    Ok(AuthenticatedStream {
        inner: stream,
        remote: AuthenticatedPeer {
            peer_id: remote_peer_id,
            subject: remote_claims.sub,
            role: remote_claims.peer_type,
            domain_ids: remote_claims.domain_ids,
        },
    })
}

async fn write_token_frame(stream: &mut Stream, token: &[u8]) -> P2PResult<()> {
    if token.len() > MAX_TOKEN_BYTES {
        return Err(Error::TokenFrameTooLarge(MAX_TOKEN_BYTES));
    }
    stream
        .write_all(&(token.len() as u32).to_be_bytes())
        .await?;
    stream.write_all(token).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_token_frame(stream: &mut Stream) -> P2PResult<Vec<u8>> {
    let mut length = [0_u8; 4];
    stream.read_exact(&mut length).await?;
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_TOKEN_BYTES {
        return Err(Error::TokenFrameTooLarge(MAX_TOKEN_BYTES));
    }
    let mut token = vec![0; length];
    stream.read_exact(&mut token).await?;
    Ok(token)
}

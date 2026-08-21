//! Generation stamping around the unchanged upstream relay client behaviour.
//!
//! The pinned upstream acceptance event is keyed only by relay Peer ID. This
//! wrapper observes the exact handler dispatch chosen for a reservation and
//! carries the Auki generation through later handler events. A canceled
//! generation can therefore never be confused with a replacement, even when
//! an old handler event was already queued inside the upstream behaviour.

use std::{
    collections::{hash_map::Entry, HashMap, VecDeque},
    task::{Context, Poll},
};

use libp2p::{
    core::{transport::PortUse, Endpoint},
    relay,
    swarm::{
        ConnectionClosed, ConnectionDenied, ConnectionId, DialError, DialFailure, FromSwarm,
        NetworkBehaviour, NotifyHandler, THandler, THandlerInEvent, THandlerOutEvent, ToSwarm,
    },
    Multiaddr, PeerId,
};

use crate::relay::RelayReservationHandle;

type Upstream = relay::client::Behaviour;
type UpstreamAction = ToSwarm<relay::client::Event, THandlerInEvent<Upstream>>;
type Action = ToSwarm<Event, THandlerInEvent<Upstream>>;

#[derive(Debug)]
pub(crate) enum Event {
    ReservationDispatched {
        handle: RelayReservationHandle,
    },
    ReservationDispatchFailed {
        handle: RelayReservationHandle,
        reason: &'static str,
    },
    Upstream {
        event: relay::client::Event,
        handle: Option<RelayReservationHandle>,
    },
}

pub(crate) struct Behaviour {
    inner: Upstream,
    pending_dispatches: HashMap<PeerId, PendingDispatch>,
    dispatches_by_connection: HashMap<ConnectionId, RelayReservationHandle>,
    handler_contexts: VecDeque<HandlerContext>,
    queued_actions: VecDeque<QueuedAction>,
}

impl Behaviour {
    pub(crate) fn new(inner: Upstream) -> Self {
        Self {
            inner,
            pending_dispatches: HashMap::new(),
            dispatches_by_connection: HashMap::new(),
            handler_contexts: VecDeque::new(),
            queued_actions: VecDeque::new(),
        }
    }

    pub(crate) fn register_dispatch(
        &mut self,
        handle: RelayReservationHandle,
        connection_id: ConnectionId,
    ) -> Result<(), ()> {
        match self.pending_dispatches.entry(handle.relay_peer_id()) {
            Entry::Vacant(entry) => {
                entry.insert(PendingDispatch {
                    handle,
                    connection_id,
                    canceled: false,
                });
                Ok(())
            }
            Entry::Occupied(_) => Err(()),
        }
    }

    pub(crate) fn has_pending_dispatch(&self, relay_peer_id: PeerId) -> bool {
        self.pending_dispatches.contains_key(&relay_peer_id)
    }

    pub(crate) fn fence_dispatch(&mut self, handle: RelayReservationHandle) -> bool {
        let mut pending = false;
        for dispatch in self.pending_dispatches.values_mut() {
            if dispatch.handle == handle {
                dispatch.canceled = true;
                pending = true;
            }
        }
        self.dispatches_by_connection
            .retain(|_, dispatched| *dispatched != handle);
        self.queued_actions
            .retain(|queued| queued.handle != Some(handle));
        pending
    }

    fn intercept(&mut self, action: UpstreamAction) -> Action {
        match action {
            ToSwarm::GenerateEvent(event) => {
                let context = self.handler_contexts.pop_front();
                ToSwarm::GenerateEvent(Event::Upstream {
                    event,
                    handle: context.and_then(|context| context.handle),
                })
            }
            ToSwarm::NotifyHandler {
                peer_id,
                handler,
                event,
            } => {
                let Some(pending) = self.pending_dispatches.remove(&peer_id) else {
                    return ToSwarm::NotifyHandler {
                        peer_id,
                        handler,
                        event,
                    };
                };
                if pending.canceled {
                    return ToSwarm::GenerateEvent(Event::ReservationDispatchFailed {
                        handle: pending.handle,
                        reason: "reservation was canceled before handler dispatch",
                    });
                }
                let selected = match handler {
                    NotifyHandler::One(connection_id) if connection_id == pending.connection_id => {
                        connection_id
                    }
                    _ => {
                        return ToSwarm::GenerateEvent(Event::ReservationDispatchFailed {
                            handle: pending.handle,
                            reason: "upstream selected a different direct relay connection",
                        })
                    }
                };
                self.dispatches_by_connection
                    .insert(selected, pending.handle);
                self.queued_actions.push_back(QueuedAction {
                    handle: Some(pending.handle),
                    action: ToSwarm::NotifyHandler {
                        peer_id,
                        handler: NotifyHandler::One(selected),
                        event,
                    },
                });
                ToSwarm::GenerateEvent(Event::ReservationDispatched {
                    handle: pending.handle,
                })
            }
            ToSwarm::Dial { opts } => {
                let peer_id = opts.get_peer_id();
                let connection_id = opts.connection_id();
                let pending = peer_id.and_then(|peer_id| self.pending_dispatches.remove(&peer_id));
                if let Some(pending) = pending {
                    // The selected connection disappeared before the queued
                    // ListenReq was consumed. Dropping this hidden dial is the
                    // fail-closed alternative to an uncorrelated late reserve.
                    let error = DialError::Aborted;
                    self.inner
                        .on_swarm_event(FromSwarm::DialFailure(DialFailure {
                            peer_id,
                            error: &error,
                            connection_id,
                        }));
                    return ToSwarm::GenerateEvent(Event::ReservationDispatchFailed {
                        handle: pending.handle,
                        reason: if pending.canceled {
                            "reservation was canceled before hidden relay dial dispatch"
                        } else {
                            "selected direct relay connection closed before dispatch"
                        },
                    });
                }
                ToSwarm::Dial { opts }
            }
            other => other.map_out(|event| Event::Upstream {
                event,
                handle: None,
            }),
        }
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = <Upstream as NetworkBehaviour>::ConnectionHandler;
    type ToSwarm = Event;

    fn handle_established_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        local_addr: &Multiaddr,
        remote_addr: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.inner.handle_established_inbound_connection(
            connection_id,
            peer,
            local_addr,
            remote_addr,
        )
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        address: &Multiaddr,
        role_override: Endpoint,
        port_use: PortUse,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.inner.handle_established_outbound_connection(
            connection_id,
            peer,
            address,
            role_override,
            port_use,
        )
    }

    fn on_swarm_event(&mut self, event: FromSwarm) {
        if let FromSwarm::ConnectionClosed(ConnectionClosed { connection_id, .. }) = &event {
            self.dispatches_by_connection.remove(connection_id);
        }
        self.inner.on_swarm_event(event);
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        self.handler_contexts.push_back(HandlerContext {
            handle: self.dispatches_by_connection.get(&connection_id).copied(),
        });
        self.inner
            .on_connection_handler_event(peer_id, connection_id, event);
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        if let Some(queued) = self.queued_actions.pop_front() {
            return Poll::Ready(queued.action);
        }
        match self.inner.poll(cx) {
            Poll::Ready(action) => Poll::Ready(self.intercept(action)),
            Poll::Pending => Poll::Pending,
        }
    }
}

struct PendingDispatch {
    handle: RelayReservationHandle,
    connection_id: ConnectionId,
    canceled: bool,
}

struct HandlerContext {
    handle: Option<RelayReservationHandle>,
}

struct QueuedAction {
    handle: Option<RelayReservationHandle>,
    action: Action,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use libp2p::{
        core::transport::ListenerId,
        swarm::{dial_opts::DialOpts, ConnectionId, ToSwarm},
        PeerId,
    };

    use crate::relay::{
        ExpectedRelayLimits, RelayProvider, RelayReservationHandle, RelayReservationNode,
    };

    use super::{Behaviour, Event};

    #[test]
    fn canceled_dispatch_tombstone_drains_hidden_dial_before_replacement() {
        let local_peer_id = PeerId::random();
        let relay_peer_id = PeerId::random();
        let (_, upstream) = libp2p::relay::client::new(local_peer_id);
        let mut behaviour = Behaviour::new(upstream);
        let old = handle(local_peer_id, relay_peer_id);
        let selected = ConnectionId::new_unchecked(11);

        behaviour.register_dispatch(old, selected).unwrap();
        assert!(behaviour.fence_dispatch(old));
        assert!(behaviour.has_pending_dispatch(relay_peer_id));

        let replacement = handle(local_peer_id, relay_peer_id);
        assert!(behaviour
            .register_dispatch(replacement, ConnectionId::new_unchecked(12))
            .is_err());

        let hidden_dial = DialOpts::peer_id(relay_peer_id).build();
        let action = behaviour.intercept(ToSwarm::Dial { opts: hidden_dial });
        assert!(matches!(
            action,
            ToSwarm::GenerateEvent(Event::ReservationDispatchFailed {
                handle,
                reason: "reservation was canceled before hidden relay dial dispatch",
            }) if handle == old
        ));
        assert!(!behaviour.has_pending_dispatch(relay_peer_id));
        behaviour
            .register_dispatch(replacement, ConnectionId::new_unchecked(12))
            .unwrap();
    }

    fn handle(local_peer_id: PeerId, relay_peer_id: PeerId) -> RelayReservationHandle {
        let limits = ExpectedRelayLimits::new(Duration::from_secs(90), 1_048_576).unwrap();
        let provider = RelayProvider::new(
            relay_peer_id,
            [format!(
                "/dns4/relay.testnet.aukiverse.com/tcp/443/p2p/{relay_peer_id}"
            )],
            limits,
        )
        .unwrap();
        RelayReservationNode::new(local_peer_id)
            .begin(provider, ListenerId::next())
            .unwrap()
    }
}

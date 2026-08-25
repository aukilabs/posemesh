# auki-p2p

`auki-p2p` is the shared authenticated P2P runtime. It owns:

- the persistent libp2p identity and peer ID;
- DDS-token verification and the process-local credential authority;
- mutually authenticated application streams;
- exact direct/circuit route opening and cancellation-safe circuit cleanup;
- relay reservation transport primitives; and
- the process-shared, authority-fenced `RouteCatalog`.

It deliberately does not own DDS/DMS HTTP clients, task scheduling, or an
application protocol's wire format.

## Adding an application protocol

Create a sibling crate such as `auki-p2p-example` and keep all protocol-specific
messages and policy there. The protocol crate should:

1. Define one versioned `ApplicationProtocol` and its inbound
   `SessionRequirements`.
2. Start its inbound endpoint with `Node::serve(ProtocolSpec, ...)`.
3. Open outbound connections with `Node::open_exact_route(...)`; never expose a
   raw `Node` or token to business logic.
4. Consume a shared `RouteCatalog` when it needs advertised direct or relay
   routes.
5. Export a narrow, cloneable service facade for its callers.

The host application remains the composition root: it acquires and refreshes
credentials, owns shutdown, constructs one `Node`/`P2pCredentialStore`/
`RouteCatalog`, and gives those shared capabilities to each protocol crate.

`auki-p2p-dataset` is the reference implementation of this pattern.

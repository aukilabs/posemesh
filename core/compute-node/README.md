# posemesh-compute-node

`posemesh-compute-node` hosts the node engine and all reusable infrastructure that
the binary crate (`bin`) wires together. The crate is responsible for
loading configuration, authenticating with DDS, polling DMS for work, managing
sessions, streaming heartbeats, and brokering storage traffic to the domain
server on behalf of capability-specific runners. Legacy SIWE and robot machine
authentication use separate, explicit entrypoints while sharing the task
engine.

The authenticated P2P stack is intentionally split out of this crate:
the SDK-owned
[`auki-p2p`](https://github.com/aukilabs/auki-sdk/tree/main/crates/auki-p2p)
owns the reusable runtime, identity, mutual authentication, relay transport,
and route catalog; the Posemesh-owned
[`auki-p2p-dataset`](../auki-p2p-dataset/README.md) owns the dataset protocol.
Compute-node remains the composition root for DDS credential acquisition, DMS
relay bookings, task lifecycle, and shutdown. Runners receive protocol-specific
facades explicitly in their constructors, never through `TaskCtx` and never as
the raw P2P node or credentials.

## Responsibilities
- Environment-driven configuration (`config`) with typed accessors and sane
  defaults where permitted.
- Telemetry bootstrap (`telemetry`) that installs a `tracing` subscriber and
  exposes helper spans.
- DDS registration helpers (`dds::register`) and the in-memory persistence stub
  used by legacy registration callbacks (`dds::persist`).
- Authentication state machines for SIWE after registration and opt-in robot
  machine authentication (`auth` module).
- DMS HTTP clients (`dms::client`, `dms::relay`) plus strict request/response
  data contracts.
- Robot-owned relay-booking coordination, confirmed Circuit Relay v2 route
  publication, and fenced child cancellation (`relay_booking`).
- Storage façade that turns leases into runner-facing input/output ports
  (`storage::{input, output, client, token}`).
- Session lifecycle management and heartbeat scheduling (`session`, `heartbeat`,
  `engine::HeartbeatDriver`).
- Poller backoff helpers (`poller`) and top-level execution loop (`engine`).
- (Legacy) HTTP router for DDS callbacks (`http`); compute nodes no longer
  need to expose inbound endpoints.

## Runtime flow (engine overview)
1. `telemetry::init_from_env()` installs logging based on `LOG_FORMAT`.
2. The selected entrypoint loads either `NodeConfig` for legacy SIWE or the
   separate `RobotNodeConfig` for robot machine authentication.
3. The engine starts process protocols, then a `RunnerComposition` constructs
   protocol-aware runners with typed dependencies such as `DatasetService`.
   Plain runners can still be registered directly in a `RunnerRegistry`.
4. The legacy entrypoint starts
   `dds::register::spawn_registration_if_configured()` and then
   `auth::SiweAfterRegistration`. The robot entrypoint registers directly with
   its opaque DDS-issued credential and never starts legacy registration or
   falls back to SIWE.
5. When Robot relay mode is enabled, the host starts the endpoint P2P-token
   refresher and relay-booking coordinator after machine authentication,
   process Peer-ID binding, and Domain assignment. The coordinator reserves
   each DMS-ready child and exposes only confirmed routes to the dataset layer.
6. The main `run_node` loop obtains an access token from DDS, builds a DMS
   client, leases tasks, initializes session state, and dispatches to the
   correct runner via `RunnerRegistry::run_for_lease`.
7. `HeartbeatDriver` coalesces progress updates and posts heartbeats on the TTL
   schedule computed by `session::HeartbeatPolicy`, refreshing storage tokens
   when DDS returns new ones.
8. When a runner finishes, artifacts discovered by the storage layer are
   reported to DMS via `complete` or `fail`, and the cycle restarts.

## Configuration surface

Required environment variables for the legacy SIWE entrypoint:
- `REG_SECRET` — shared secret issued by DDS during provisioning.
- `SECP256K1_PRIVHEX` — 32-byte hex-encoded private key used to sign SIWE
  messages.

Required credential source for the explicit robot entrypoint (configure exactly
one):
- `ROBOT_REGISTRATION_CREDENTIALS` — the complete opaque credential returned
  once when DDS provisions or rotates the robot.
- `ROBOT_REGISTRATION_CREDENTIALS_FILE` — path to a file containing that opaque
  credential. This is the preferred handoff from an enrollment wrapper or
  container init step using a persistent secret volume.

Do not decode, split, log, or commit the credential. The file loader trims a
trailing newline and fails closed when the path is unreadable or the file is
empty. Updating the file requires restarting the compute node.

Optional environment variables:
- `DMS_BASE_URL` (default `https://dms.auki.network/v1`) — base URL of the DMS
  REST API.
- `DDS_BASE_URL` (default `https://dds.auki.network`) — base URL of the DDS API
  used by the selected authentication flow.
- `REQUEST_TIMEOUT_SECS` (default `60`) — per-request timeout applied to DDS
  authentication and ordinary task DMS calls. Relay control requests use their
  fixed 10-second timeout described below.
- `NODE_VERSION` (default crate version) — optional override for the advertised
  node version.
- `HEARTBEAT_JITTER_MS` (default `250`) — backoff applied when coalescing
  heartbeat updates for the legacy scheduler in `heartbeat::run_scheduler`.
- `HEARTBEAT_MIN_RATIO` / `HEARTBEAT_MAX_RATIO` (defaults `0.25` / `0.35`) —
  fraction of the lease TTL after which the engine schedules the next heartbeat.
- `POLL_BACKOFF_MS_MIN` / `POLL_BACKOFF_MS_MAX` (defaults `1000` / `30000`) —
  jitter range used between idle lease polls.
- `TOKEN_SAFETY_RATIO` (default `0.75`) — access-token renewal threshold.
- `TOKEN_REAUTH_MAX_RETRIES` (default `3`) — retries before bailing on token
  refresh.
- `TOKEN_REAUTH_JITTER_MS` (default `500`) — jitter applied between retries.
- `AUKI_P2P_ENABLED` (default `false`) — enables the process-level P2P runtime,
  including libp2p identity binding and DDS P2P-token refresh.
  Selecting relay mode `auto` or `always` also enables this runtime.
- `AUKI_P2P_PRIVATE_KEY_FILE` (required when P2P or relay booking is enabled;
  preferred over inline configuration) — path to a raw canonical
  Ed25519 libp2p protobuf private key. The file must be a regular file of at
  most 4 KiB with no group/other permission bits (normally mode `0600`).
- `AUKI_P2P_PRIVATE_KEY` (alternative required form) — canonical padded RFC
  4648 Base64 of the same protobuf bytes. It is mutually exclusive with
  `AUKI_P2P_PRIVATE_KEY_FILE`. If neither value is configured, production P2P
  startup fails closed; it never substitutes an ephemeral identity.
- `AUKI_P2P_LISTEN_MULTIADDRS` (default empty) — comma-separated native TCP
  multiaddrs for the process-level libp2p node. Direct-only Robot serving
  requires at least one explicit value; `auto` and `always` may leave it empty
  and become ready through a confirmed circuit listener. Tests may use
  `/ip4/127.0.0.1/tcp/0`.
- `AUKI_P2P_ADVERTISED_MULTIADDRS` (default empty) — comma-separated TCP
  multiaddrs placed in dataset references. Direct-only Robot serving requires
  explicit addresses that Compute Nodes can reach. `auto` and `always` may
  leave this empty, but cannot register a dataset until a relay route is
  confirmed. There is no direct-address discovery or guessing, and an
  ephemeral `tcp/0` address must not be advertised.
- `AUKI_P2P_RELAY_MODE` (Robot only; default `disabled`) — one of `disabled`,
  `auto`, or `always`; see the readiness rules below.
- `AUKI_P2P_RELAY_BOOKING_MODE` (Robot only; default `public`) — `public`
  permits an eligible public relay, while `dedicated` restricts selection to
  the Robot's organization. DMS never relaxes the selected policy to fill a
  shortfall.
- `AUKI_P2P_RELAY_BOOKING_DURATION_SECONDS` (Robot only; default `86400`) —
  requested rolling horizon, accepted range `300..=86400`.
- `AUKI_P2P_RELAY_COUNT` (Robot only; default `1`) — desired distinct relay
  children, accepted range `1..=3`.
- `AUKI_P2P_RELAY_STATUS_POLL_INTERVAL_SECONDS` (Robot only; default `5`) —
  child-status cadence, accepted whole-second range `1..=60`.
- `REGISTER_INTERVAL_SECS` (legacy SIWE only; default `120`) — cooldown between
  registration attempts while the node is not yet registered or is recovering.
- `REGISTER_MAX_RETRY` (legacy SIWE only; default `-1`, meaning infinite
  retries) — retry cap for transient registration failures before falling back
  to the regular cooldown.
- `MAX_CONCURRENCY` (default `1`) — staging knob for future multi-runner
  concurrency.
- `LOG_FORMAT` (default `json`) — set to `text` for pretty console logs.
- `ENABLE_NOOP` (default `false`) — when true the binary registers noop runners.
- `NOOP_SLEEP_SECS` (default `5`) — noop runner sleep duration.

Relay control timing is fixed in v1 rather than exposed as more environment
variables: a 10-second HTTP timeout, jittered retry delays from 250 milliseconds
through 5 seconds, a 30-second reservation retry budget, and a 15-second
authority-deadline safety margin. Startup also rejects configurations whose
direct advertised-route count plus requested relay count could exceed the
16-route reference limit. At most three of those routes may be circuits.

### Robot relay readiness and shutdown

`disabled` never creates a relay booking. When P2P serving is enabled in this
mode, operators must supply explicit listen and advertised direct routes and
accept that an immutable direct-only reference cannot be repaired if the route
is private, stale, blackholed, or later becomes unreachable.

`auto` always books a relay fallback. With a configured direct advertised
route, task polling may begin before the relay is ready, but immutable dataset
registration still waits for at least one eligible confirmed relay. Without a
direct route, task polling also waits for the first confirmation. `always`
gates both task polling and registration on a confirmation even when direct
routes are configured.

`AUKI_P2P_RELAY_COUNT` is desired redundancy, not a quorum. One confirmed child
is usable when two or three were requested; queued, recovering, or replacement
siblings continue in the background without changing booking mode. Zero
confirmed eligible children blocks relay-backed registration. Each immutable
reference snapshots its explicit direct routes plus the confirmed,
dataset-limit-eligible relay routes available at that commit. A child confirmed
later appears only in future references, and a reassigned route is not
retrofitted into an old reference.

Graceful Robot shutdown stops new dataset registrations first while keeping the
shared endpoint token, booking authority, and still-authorized reservations
alive through the greatest outstanding `available_until` and the existing
15-minute limit for an already-open transfer attempt. It then exact-owner
deletes the parent booking. Forced process termination or an unrecoverable
coordinator failure cannot provide that drain and may break already-published
immutable references. The required persistent P2P key lets a replacement prove
the same Peer ID and reconcile its peer-bound relay booking instead of waiting
for prior authority to expire. It does not reconstruct the old process's
in-memory dataset registrations or repair references whose availability already
ended. Exactly one live process may own a given P2P private key at a time.

Generate a dedicated Ed25519 identity without printing the private key:

```sh
cargo run -p posemesh-compute-node --bin posemesh-p2p-keygen -- \
  "$HOME/.auki/robot/libp2p-private-key"
```

The generator refuses to overwrite a path and creates the raw key with mode
`0600`. Configure that path as `AUKI_P2P_PRIVATE_KEY_FILE`. Do not reuse a SIWE,
wallet, or registration private key for libp2p identity.

Relay policy and credentials remain host-only. The coordinator uses the
peer-bound Robot machine JWT for DMS booking calls and a separate endpoint P2P
token for end-to-end authentication. Neither the runner nor `TaskCtx` receives
machine, booking, relay, or P2P credentials, and relay count does not alter the
ordinary runner `MAX_CONCURRENCY` setting.

## Hello runner entrypoints

The existing command remains the default and continues to use SIWE:

```sh
cd core
cargo run -p posemesh-hello-runner
```

Robot mode is deliberately a different binary. From the `core` workspace, copy
the placeholder without overwriting any existing legacy SIWE `.env`, insert the
one-time credential returned by DDS, and invoke that binary by name:

```sh
cp -n compute-node-runner-api/examples/hello-runner/.env.robot.example \
  compute-node-runner-api/examples/hello-runner/.env
cargo run -p posemesh-hello-runner --bin posemesh-robot-hello-runner
```

The example advertises `/examples/hello/v1`. A disposable local DDS must have
that capability catalogued with `public_url_required = false` and the robot
must be provisioned with the same capability. The example does not add or
enable a production DDS capability.

Choosing a binary is the authentication-mode switch. Supplying either robot
credential source to the default SIWE binary does not select robot mode, and
the robot binary does not read `REG_SECRET` or `SECP256K1_PRIVHEX`.

## Notable modules
- `auth::siwe_after_registration` — waits for DDS registration, then spins up
  the SIWE token manager and refresh loop.
- `auth::robot` — registers and renews robot machine credentials without a
  wallet, private key, or SIWE fallback.
- `dds::register` — normalizes versions (stripping leading `v`), validates the
  secp256k1 key, and launches the registration task using
  `posemesh-node-registration`. Once acquired, registration is parked until the
  runtime needs recovery instead of being refreshed on a fixed cadence.
- `engine` — orchestrates leasing, cancellation, heartbeat posting, and
  completion/failure reporting, plus Robot relay startup and graceful drain.
  The `RunnerRegistry` façade makes it easy to add new capabilities without
  exposing relay credentials.
- `dms::relay` — strict, body-redacting typed client for active/create/renew,
  reservation-failure, and exact-owner delete operations.
- `relay_booking` — serializes parent booking calls, reconciles stable child
  fences, and publishes only locally confirmed reservations.
- `storage::client` — performs authenticated multipart downloads/uploads
  against the domain server using safe temporary directories.
- `session` — tracks lease metadata, computes TTL-driven heartbeat deadlines,
  and survives new heartbeats refreshing tokens or signalling cancellation.

## Developing and testing
- Run `cargo test -p posemesh-compute-node` to exercise storage, session, and DDS
  registration behaviour.
- The crate uses Tokio throughout; tests rely on the multi-threaded runtime,
  so avoid enabling the single-threaded scheduler when adding new async tests.
- `LOG_FORMAT=text` is useful during local development to keep logs readable.
- The HTTP router is legacy; compute nodes do not require inbound callbacks.

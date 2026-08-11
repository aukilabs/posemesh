# posemesh-compute-node

`posemesh-compute-node` hosts the node engine and all reusable infrastructure that
the binary crate (`bin`) wires together. The crate is responsible for
loading configuration, authenticating with DDS, polling DMS for work, managing
sessions, streaming heartbeats, and brokering storage traffic to the domain
server on behalf of capability-specific runners. Legacy SIWE and robot machine
authentication use separate, explicit entrypoints while sharing the task
engine.

## Responsibilities
- Environment-driven configuration (`config`) with typed accessors and sane
  defaults where permitted.
- Telemetry bootstrap (`telemetry`) that installs a `tracing` subscriber and
  exposes helper spans.
- DDS registration helpers (`dds::register`) and the in-memory persistence stub
  used by legacy registration callbacks (`dds::persist`).
- Authentication state machines for SIWE after registration and opt-in robot
  machine authentication (`auth` module).
- DMS HTTP client (`dms::client`) plus request/response data contracts.
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
3. Runners are registered in a `RunnerRegistry`; the binary decides which
   capabilities to advertise.
4. The legacy entrypoint starts
   `dds::register::spawn_registration_if_configured()` and then
   `auth::SiweAfterRegistration`. The robot entrypoint registers directly with
   its opaque DDS-issued credential and never starts legacy registration or
   falls back to SIWE.
5. The main `run_node` loop obtains an access token from DDS, builds a DMS
   client, leases tasks, initializes session state, and dispatches to the
   correct runner via `RunnerRegistry::run_for_lease`.
6. `HeartbeatDriver` coalesces progress updates and posts heartbeats on the TTL
   schedule computed by `session::HeartbeatPolicy`, refreshing storage tokens
   when DDS returns new ones.
7. When a runner finishes, artifacts discovered by the storage layer are
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
  authentication and DMS calls.
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
  completion/failure reporting. The `RunnerRegistry` façade makes it easy to add
  new capabilities.
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

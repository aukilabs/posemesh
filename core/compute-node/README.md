# posemesh-compute-node

`posemesh-compute-node` hosts the node engine and all reusable infrastructure that
the binary crate (`bin`) wires together. The crate is responsible for
loading configuration, authenticating with DDS, polling DMS for work, managing
sessions, streaming heartbeats, and brokering storage traffic to the domain
server on behalf of capability-specific runners.

## Responsibilities
- Environment-driven configuration (`config`) with typed accessors and sane
  defaults where permitted.
- Telemetry bootstrap (`telemetry`) that installs a `tracing` subscriber and
  exposes helper spans.
- DDS registration helpers (`dds::register`) and the in-memory persistence stub
  used by legacy registration callbacks (`dds::persist`).
- Authentication state machine for SIWE after registration (`auth` module).
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
2. `NodeConfig::from_env()` loads operational settings. The node currently
   requires DDS configuration (see below) because SIWE tokens are mandatory.
3. Runners are registered in a `RunnerRegistry`; the binary decides which
   capabilities to advertise.
4. `dds::register::spawn_registration_if_configured()` starts the outbound
   SIWE-based registration loop, and `auth::SiweAfterRegistration` waits for a
   successful registration before requesting access tokens.
5. The main `run_node` loop obtains an access token from DDS, builds a DMS
   client, leases tasks, initializes session state, and dispatches to the
   correct runner via `RunnerRegistry::run_for_lease`.
6. `HeartbeatDriver` coalesces progress updates and posts heartbeats on the TTL
   schedule computed by `session::HeartbeatPolicy`, refreshing storage tokens
   when DDS returns new ones.
7. When a runner finishes, artifacts discovered by the storage layer are
   reported to DMS via `complete` or `fail`, and the cycle restarts.

## Configuration surface

Required environment variables:
- `REG_SECRET` — shared secret issued by DDS during provisioning.
- `SECP256K1_PRIVHEX` — 32-byte hex-encoded private key used to sign SIWE
  messages.

Optional environment variables:
- `DMS_BASE_URL` (default `https://dms.auki.network/v1`) — base URL of the DMS
  REST API.
- `DDS_BASE_URL` (default `https://dds.auki.network`) — base URL of the DDS API
  (used for SIWE authentication).
- `REQUEST_TIMEOUT_SECS` (default `60`) — per-request timeout applied to DMS
  calls.
- `NODE_VERSION` (default crate version) — optional override for the advertised
  node version.
- `HEARTBEAT_JITTER_MS` (default `250`) — backoff applied when coalescing
  heartbeat updates for the legacy scheduler in `heartbeat::run_scheduler`.
- `HEARTBEAT_MIN_RATIO` / `HEARTBEAT_MAX_RATIO` (defaults `0.25` / `0.35`) —
  fraction of the lease TTL after which the engine schedules the next heartbeat.
- `POLL_BACKOFF_MS_MIN` / `POLL_BACKOFF_MS_MAX` (defaults `1000` / `30000`) —
  jitter range used between idle lease polls.
- `TOKEN_SAFETY_RATIO` (default `0.75`) — SIWE token renewal threshold.
- `TOKEN_REAUTH_MAX_RETRIES` (default `3`) — retries before bailing on token
  refresh.
- `TOKEN_REAUTH_JITTER_MS` (default `500`) — jitter applied between retries.
- `REGISTER_INTERVAL_SECS` (default `120`) — DDS registration loop cadence.
- `REGISTER_MAX_RETRY` (default `-1`, meaning infinite retries) — DDS
  registration retry cap.
- `MAX_CONCURRENCY` (default `1`) — staging knob for future multi-runner
  concurrency.
- `LOG_FORMAT` (default `json`) — set to `text` for pretty console logs.
- `ENABLE_NOOP` (default `false`) — when true the binary registers noop runners.
- `NOOP_SLEEP_SECS` (default `5`) — noop runner sleep duration.

## Notable modules
- `auth::siwe_after_registration` — waits for DDS registration, then spins up
  the SIWE token manager and refresh loop.
- `dds::register` — normalizes versions (stripping leading `v`), validates the
  secp256k1 key, and launches the registration task using `posemesh-node-registration`.
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

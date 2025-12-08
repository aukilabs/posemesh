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
  used by the registration callback (`dds::persist`).
- Authentication state machine for SIWE after registration (`auth` module).
- DMS HTTP client (`dms::client`) plus request/response data contracts.
- Storage façade that turns leases into runner-facing input/output ports
  (`storage::{input, output, client, token}`).
- Session lifecycle management and heartbeat scheduling (`session`, `heartbeat`,
  `engine::HeartbeatDriver`).
- Poller backoff helpers (`poller`) and top-level execution loop (`engine`).
- Lightweight HTTP server exposing `/health` and the registration callback used
  by DDS (`http`).

## Runtime flow (engine overview)
1. `telemetry::init_from_env()` installs logging based on `LOG_FORMAT`.
2. The HTTP router is started so DDS can POST the registration secret once the
   node signs in.
3. `NodeConfig::from_env()` loads operational settings. The node currently
   requires DDS configuration (see below) because SIWE tokens are mandatory.
4. Runners are registered in a `RunnerRegistry`; the binary decides which
   capabilities to advertise.
5. `dds::register::spawn_registration_if_configured()` starts the outbound
   registration loop, and `auth::SiweAfterRegistration` blocks until the DDS
   callback persists the registration secret.
6. The main `run_node` loop obtains an access token from DDS, builds a DMS
   client, leases tasks, initializes session state, and dispatches to the
   correct runner via `RunnerRegistry::run_for_lease`.
7. `HeartbeatDriver` coalesces progress updates and posts heartbeats on the TTL
   schedule computed by `session::HeartbeatPolicy`, refreshing storage tokens
   when DDS returns new ones.
8. When a runner finishes, artifacts discovered by the storage layer are
   reported to DMS via `complete` or `fail`, and the cycle restarts.

## Configuration surface

Required environment variables:
- `DMS_BASE_URL` — base URL of the DMS REST API.
- `REQUEST_TIMEOUT_SECS` — per-request timeout applied to DMS calls.
- `NODE_VERSION` — optional override for the advertised node version; defaults
  to the crate version when unset or blank.
- `DDS_BASE_URL` — base URL of the DDS API (used for SIWE authentication).
- `NODE_URL` — externally reachable URL of this node; sent to DDS.
- `REG_SECRET` — shared secret issued by DDS during provisioning.
- `SECP256K1_PRIVHEX` — 32-byte hex-encoded private key used to sign SIWE
  messages.

Optional environment variables:
- `HEARTBEAT_JITTER_MS` (default `250`) — backoff applied when coalescing
  heartbeat updates for the legacy scheduler in `heartbeat::run_scheduler`.
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
- `POSEMESH_MULTIPART_MIN_BYTES` (default `8388608`) — size threshold (bytes)
  after which artifact uploads switch to the domain server's multipart API;
  useful for large outputs like `refined/global/refined_sfm_combined/*`.

## Notable modules
- `auth::siwe_after_registration` — waits for DDS registration to persist a
  secret, then spins up the SIWE token manager and refresh loop.
- `dds::register` — normalizes versions (stripping leading `v`), validates the
  secp256k1 key, and launches the registration task using `posemesh-node-registration`.
- `engine` — orchestrates leasing, cancellation, heartbeat posting, and
  completion/failure reporting. The `RunnerRegistry` façade makes it easy to add
  new capabilities.
- `storage::client` — performs authenticated downloads/uploads (including
  multipart) against the domain server using safe temporary directories.
- `storage::output` — implements `ArtifactSink` with size-aware routing to
  multipart uploads and supports streaming via `open_multipart`.
- `session` — tracks lease metadata, computes TTL-driven heartbeat deadlines,
  and survives new heartbeats refreshing tokens or signalling cancellation.

## Developing and testing
- Run `cargo test -p posemesh-compute-node` to exercise storage, session, and DDS
  registration behaviour.
- The crate uses Tokio throughout; tests rely on the multi-threaded runtime,
  so avoid enabling the single-threaded scheduler when adding new async tests.
- `LOG_FORMAT=text` is useful during local development to keep logs readable.
- The HTTP router is minimal by design. If you extend it, keep the registration
  handler backwards compatible with the DDS callback contract.

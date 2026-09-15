# Configuration reference

The binaries call `NodeConfig::from_env()` or `RobotNodeConfig::from_env()`.
They read exported process variables; sourcing an environment file is the
caller's responsibility. Parsing and defaults are defined in
[`config.rs`](../../core/compute-node/src/config.rs).

## Required identity

| Host | Variables |
| --- | --- |
| Compute | `REG_SECRET`: complete encoded node registration credentials; `SECP256K1_PRIVHEX`: wallet signing private key in hex |
| Robot | Exactly one of `ROBOT_REGISTRATION_CREDENTIALS` and `ROBOT_REGISTRATION_CREDENTIALS_FILE` |
| Either, with P2P enabled | Exactly one of `AUKI_P2P_PRIVATE_KEY_FILE` and `AUKI_P2P_PRIVATE_KEY` |

[Provisioning](../how-to/provision-workers.md) explains how to obtain worker
credentials. [Configure workers](../how-to/configure-workers.md) shows how to
pass them to a host.
Robot and P2P files are read at startup; replacing a file requires restarting
the host. Robot credentials are opaque and whitespace is trimmed on load.

P2P key files contain raw Ed25519 libp2p protobuf bytes, must be regular files
of 1–4096 bytes, and must have mode `0600` on Unix. The inline alternative is
canonical padded Base64 of those bytes. Generate a persistent file with
`posemesh-p2p-keygen`, as shown in the [tutorial](../tutorials/robot-and-compute.md).
Keep worker identities distinct and preserve keys across restarts.

## Robot audience

`DDS_ROBOT_AUDIENCE` is optional for the official DDS endpoints below. When
unset, the SDK selects the expected JWT audience from `DDS_BASE_URL`:

| `DDS_BASE_URL` | Robot audience |
| --- | --- |
| `https://dds.dev.aukiverse.com` | `https://dds.dev.aukiverse.com/robots` |
| `https://dds.staging.aukiverse.com` | `https://dds.staging.aukiverse.com/robots` |
| `https://dds.auki.network` | `https://dds.auki.network/robots` |

These are exact root HTTPS endpoints; a trailing slash is accepted. Custom
hosts, path prefixes and nonstandard ports require an explicit audience from
the DDS deployment configuration. Set `DDS_ROBOT_AUDIENCE` or call
`RobotNodeConfig::set_audience` to override the default. Empty or
whitespace-containing overrides fail instead of selecting a default.

The audience is a token identifier, not a robot-listing endpoint. It must be
exclusive to robots and differ from the normal DDS audience. Never infer it
from an unverified token. A preset does not imply deployed robot support;
check the [deployment requirements](../how-to/provision-workers.md#check-deployment-support).

## Shared settings

| Variable | Default | Meaning |
| --- | --- | --- |
| `DDS_BASE_URL` | `https://dds.auki.network` | DDS root endpoint |
| `DMS_BASE_URL` | `https://dms.auki.network/v1` | DMS API base, including `/v1` |
| `REQUEST_TIMEOUT_SECS` | `60` | Machine/DMS request timeout, 1–300 seconds; managed Domain transfers use 30 seconds per request |
| `NODE_VERSION` | Host crate version | Version reported by the worker |
| `LOG_FORMAT` | `json` | `json` or `text` |
| `RUST_LOG` | `info` | Tracing filter when initializing the provided telemetry |
| `CLIENT_ID` | `posemesh-compute-node/<random UUID>` | Identifier shared by the managed credential and task data clients |
| `POLL_BACKOFF_MS_MIN` / `POLL_BACKOFF_MS_MAX` | `1000` / `30000` | Idle polling backoff bounds |


Endpoint defaults target production. Set both explicitly for another
environment. Application login additionally needs its matching API endpoint;
the worker itself authenticates through DDS.

Compute registration uses `REGISTER_INTERVAL_SECS=120`. The SDK bounds transient
registration failures to three attempts; terminal failure stops the host. Robot
presence registration repeats every 120 seconds. Both are awaited on shutdown.

The SDK schedules heartbeats at half the remaining authority/request window,
capped at 30 seconds, and wakes early for progress, events or rejected data
credentials. It owns serialized machine renewal and a single authenticated
retry after DMS HTTP 401.

`HEARTBEAT_MIN_RATIO`, `HEARTBEAT_MAX_RATIO`, `HEARTBEAT_JITTER_MS`,
`TOKEN_SAFETY_RATIO`, `TOKEN_REAUTH_MAX_RETRIES`, `TOKEN_REAUTH_JITTER_MS` and
`REGISTER_MAX_RETRY` remain parsed to preserve host configuration compatibility;
they do not tune the managed entrypoints. `MAX_CONCURRENCY`, `ENABLE_NOOP`
and `NOOP_SLEEP_SECS` remain compatibility fields: they do not enable concurrent
tasks or install a runner. The managed host executes one lease at a time.

## P2P

| Variable | Default | Meaning |
| --- | --- | --- |
| `AUKI_P2P_ENABLED` | `false` | Enable the authenticated SDK peer |
| `AUKI_P2P_LISTEN_MULTIADDRS` | Empty | Comma-separated native TCP listen addresses |
| `AUKI_P2P_ADVERTISED_MULTIADDRS` | Empty | Comma-separated reachable direct TCP routes |

Compute uses outbound connections and has a peer context only during an
authorized task. Its host does not book a relay. Robot keeps a fixed-Domain
peer for the process lifetime. Direct-only robot operation needs listen and
advertised addresses; advertised routes must have a concrete port.
An advertised address must be reachable by the other peer.

## Robot relay settings

| Variable | Default | Accepted values |
| --- | --- | --- |
| `AUKI_P2P_RELAY_MODE` | Enabled when P2P is enabled | `disabled`, `auto`, `always`; `auto` and `always` currently both enable booking |
| `AUKI_P2P_RELAY_BOOKING_MODE` | `public` | `public` or `dedicated` relay pool |
| `AUKI_P2P_RELAY_BOOKING_DURATION_SECONDS` | `86400` | 300–86400 seconds |
| `AUKI_P2P_RELAY_COUNT` | `1` | 1–3 bookings |
| `AUKI_P2P_RELAY_STATUS_POLL_INTERVAL_SECONDS` | `30` | 1–60 seconds |

Relay mode `auto` or `always` requires P2P enabled. Readiness requires one
confirmed relay reservation; additional requested bookings can recover in the
background. Relay pool selection is independent of the worker's task mode.
Use the [direct TCP variation](../tutorials/robot-and-compute.md#use-direct-tcp-on-one-machine)
when a reachable direct route is sufficient.

## Echo settings

Both binaries need `ECHO_ORGANIZATION_ID` and `ECHO_DOMAIN_ID` as UUIDs.
The helper also needs `ECHO_COMPUTE_PEER_ID`, `ECHO_ROBOT_PEER_ID`, and either
`APP_JWT_FILE` or `APP_JWT` (the file takes precedence). Its default message is
`hello over P2P` and its default timeout is 120 seconds (accepted range 1–600).

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Compute cannot register or authenticate | Complete encoded `REG_SECRET`, correct wallet key, DDS endpoint and staking policy |
| Robot provisioning returns 403/404 | Operator permissions and robot feature support in that DDS deployment |
| Robot is online but claims no work | Domain assignment, matching capability, dedicated job mode, DMS robot support |
| Job creation fails or work stays queued | Worker is registered and polling; capability, mode, Domain and organization match; workers may already hold a lease |
| Job submission returns 401/403 | Fresh DDS Domain token with matching `org`/`domain_id` and write scope |
| Missing Domain or storage endpoint in lease | Compatible DMS/DDS lease contract and a Domain with a Domain Server |
| Authenticated P2P protocol surface unavailable | P2P enabled, valid persistent key, and a compute lease carrying peer-bound P2P authority for this capability |
| Robot never becomes ready | Assigned Domain, DDS P2P verification, DMS relay availability, or reachable direct listen/advertised routes |
| Echo readiness identifies another peer | Isolate the example capabilities; metadata Peer IDs do not pin DMS scheduling |
| Echo times out | Both jobs active together, same run/Domain/organization, correct robot route, bounded network reachability |

Use task/job IDs and sanitized errors when debugging. The Echo helper's
`--dry-run` checks job construction without submitting work; it does not test
authentication or backend compatibility.

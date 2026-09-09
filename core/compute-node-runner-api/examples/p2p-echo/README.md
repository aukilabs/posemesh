# DMS workers with P2P Echo

Two native Rust binaries use the existing Posemesh runner interface:

| Worker | DMS capability | P2P work |
| --- | --- | --- |
| Robot | `/examples/p2p-echo/serve/v1` | Mount the SDK Echo endpoint and wait for the expected Compute peer and message. |
| Compute | `/examples/p2p-echo/send/v1` | Open the Robot's advertised route, send the message, and verify the echoed bytes. |

Both poll DMS, heartbeat, report progress, observe cancellation, and complete
their tasks through `posemesh-compute-node`. The wire protocol is the SDK's
`auki-portable-echo` protocol, `/example/echo/1.0.0`.

`RunnerComposition::with_protocols` supplies the SDK protocol handle to each
runner constructor. Compute calls `get()` inside each task: its peer is scoped
to that lease's Domain and stops when the task ends. Robot has one peer for its
assigned Domain; the serve task mounts and closes its own Echo endpoint without
stopping that peer. No change to `Runner` or `TaskCtx` is required.

## Configuration checkpoint

Configure and review these values before starting either binary or submitting
jobs. The sample files contain placeholders and do not provision anything.

- Use one DDS/DMS environment and one organization for the Compute node,
  Robot, Domain, and job-submitting account/application.
- Provision a **dedicated Compute node** with the send capability and a Robot
  with the serve capability. Assign the Robot to the chosen Domain in DDS.
  Use a DDS version that supports third-party capabilities for both Compute
  and Robot; the example capabilities are created during provisioning or
  registration without a manual catalog entry.
  Robot workers must be enabled in DMS.
- Use isolated example workers for these capabilities. DMS schedules by
  capability and placement, not by the expected Peer IDs in task metadata.
  The example verifies those IDs when it runs.
- The DMS version must issue task P2P credentials for **every peer-bound
  Compute capability**, on claim and heartbeat. An older DMS restricted to
  reconstruction will not supply the Echo task's authority. Unbound Compute
  nodes remain HTTP-only.
- Configure the Compute registration secret and its SIWE wallet key. Configure
  the Robot's opaque DDS-issued registration credential separately.
- Give each process its own persistent libp2p key and record its public Peer ID.
  A libp2p key is separate from the Compute wallet key.
- Set the same `ECHO_ORGANIZATION_ID` and `ECHO_DOMAIN_ID` in both worker
  environments. Runners check these against the leased Domain and signed
  P2P authority. The job token must belong to that organization and Domain too.
- The Domain must have a reachable Domain server and working lease credentials.
  Echo does not upload artifacts, but the runner host still initializes its
  ordinary storage ports.
- For the default relay configuration, a compatible TCP relay provider must be
  available. If DMS credit locking is enabled, configure prices and sufficient
  credits for both example capabilities before submitting jobs.

The node's dedicated mode comes from DDS provisioning; there is no
`NODE_MODE` environment override. The helper always submits **dedicated
tasks**. Robot's `AUKI_P2P_RELAY_BOOKING_MODE=public` selects a relay provider
pool and is independent of task mode.

## Prepare the worker environments

All commands below are run from `posemesh/core`.

```sh
cp -n compute-node-runner-api/examples/p2p-echo/.env.compute.example \
  compute-node-runner-api/examples/p2p-echo/.env.compute
cp -n compute-node-runner-api/examples/p2p-echo/.env.robot.example \
  compute-node-runner-api/examples/p2p-echo/.env.robot
```

Edit the copies with the agreed configuration. The binaries use the process
environment and do not load `.env` files automatically.

If the workers do not already have dedicated libp2p keys, generate them locally
and put the resulting paths in the two environment files:

```sh
mkdir -p "$HOME/.auki/p2p-echo"
cargo run -p posemesh-compute-node --bin posemesh-p2p-keygen -- \
  "$HOME/.auki/p2p-echo/compute.key"
cargo run -p posemesh-compute-node --bin posemesh-p2p-keygen -- \
  "$HOME/.auki/p2p-echo/robot.key"
```

The utility prints the public Peer ID and refuses to overwrite a key.
Keep credentials and keys out of source control.

## Start after configuration review

In the Robot terminal:

```sh
source compute-node-runner-api/examples/p2p-echo/.env.robot
cargo run --locked -p posemesh-p2p-echo --bin posemesh-p2p-echo-robot
```

In the Compute terminal:

```sh
source compute-node-runner-api/examples/p2p-echo/.env.compute
cargo run --locked -p posemesh-p2p-echo --bin posemesh-p2p-echo-compute
```

Wait for both workers to authenticate and poll DMS. DMS checks capability
availability when jobs are created. Robot also waits for its peer to be ready
before polling; a pending relay reservation can delay that.

In a third terminal, configure `DMS_BASE_URL` (including `/v1`),
`ECHO_ORGANIZATION_ID`, and `ECHO_DOMAIN_ID` to the same agreed values.
Set the two public Peer IDs and a path to a DDS-signed Domain job token:

```sh
export ECHO_COMPUTE_PEER_ID=COMPUTE_PEER_ID
export ECHO_ROBOT_PEER_ID=ROBOT_PEER_ID
export APP_JWT_FILE=/absolute/path/to/domain-job-token
python3 compute-node-runner-api/examples/p2p-echo/scripts/submit.py --dry-run
```

`--dry-run` only prints the dedicated serve job. It does not read a token or
contact a service. Once the configuration is ready, submit:

```sh
python3 compute-node-runner-api/examples/p2p-echo/scripts/submit.py \
  --message "hello from Compute to Robot" --timeout-seconds 120
```

`APP_JWT` is also supported. The helper checks the token's organization,
Domain, and expiry locally; DMS verifies its signature and permissions.

The helper creates the Robot serve job first, waits for `phase=ready`, checks
its run ID, organization, Domain, protocol, and both Peer IDs, then copies its
public route into an independent Compute send job. A DAG completion edge
would block this interaction because Robot waits for Compute before completing.
Each run has a unique ID in the Echo payload and one attempt per task.

Success means both tasks reach `completed`: Robot reports `phase=echoed`,
Compute reports `phase=verified`, and the helper prints both results.
Failures or interruption cancel only the jobs that this helper created.
Robot also has a bounded serve timeout (1–600 seconds).

## Direct local transport

To use two processes on the same machine without booking a relay, replace the
Robot's relay setting with:

```sh
export AUKI_P2P_RELAY_MODE=disabled
export AUKI_P2P_LISTEN_MULTIADDRS=/ip4/127.0.0.1/tcp/4001
export AUKI_P2P_ADVERTISED_MULTIADDRS=/ip4/127.0.0.1/tcp/4001
```

For separate hosts, use an advertised TCP address reachable from Compute.
Compute needs outbound access and does not book its own relay. In the default
variant it opens the Robot's confirmed relay circuit. Readiness publishes a
confirmed TCP relay route when available, otherwise an advertised direct route.
Neither variant bypasses DDS authentication or DMS task scheduling.

## Local checks

```sh
make ci-compute-node
```

This compiles both binaries, runs Rust tests and offline submission-helper
tests, and checks formatting and Clippy. The engine tests use loopback
DDS/DMS fixtures. They do not start these example binaries, provision workers,
submit real jobs, or establish that a live deployment is configured correctly.

Cancel unfinished demo jobs before stopping workers. Robot closes the task's
Echo registration on cancellation, failure, or timeout. The host drains active
tasks during normal shutdown and releases its peer and relay resources.

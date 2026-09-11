# Run a robot and compute node together

Run two Rust workers that claim tasks through DMS and exchange
`hello from Compute to Robot` over P2P. Both tasks then complete in DMS.

You need Rust 1.89+, Python 3, this repository, three terminals, and
[configured workers](../how-to/configure-workers.md).
Use one organization and Domain for this example. The robot serves
`/examples/p2p-echo/serve/v1`; the dedicated compute node sends with
`/examples/p2p-echo/send/v1`.

All commands below run from **`posemesh/core`**.

## Prepare the workers

Copy the example environments and fill in your endpoints, organization,
Domain, and credentials:

~~~sh
cp -n compute-node-runner-api/examples/p2p-echo/.env.compute.example \
  compute-node-runner-api/examples/p2p-echo/.env.compute
cp -n compute-node-runner-api/examples/p2p-echo/.env.robot.example \
  compute-node-runner-api/examples/p2p-echo/.env.robot
~~~

The files contain `export` statements. The binaries read the process environment;
they do not load these files themselves.

Generate a separate persistent P2P key for each worker if it does not already
have one:

~~~sh
mkdir -p "$HOME/.auki/p2p-echo"
cargo run --locked -p posemesh-compute-node --bin posemesh-p2p-keygen -- \
  "$HOME/.auki/p2p-echo/compute.key"
cargo run --locked -p posemesh-compute-node --bin posemesh-p2p-keygen -- \
  "$HOME/.auki/p2p-echo/robot.key"
~~~

Each command prints the public Peer ID and creates a private file with mode
`0600`; it refuses to overwrite an existing path. Record both Peer IDs.
Set each environment's `AUKI_P2P_PRIVATE_KEY_FILE` to its key's absolute path.

## Start the workers

In the robot terminal:

~~~sh
source compute-node-runner-api/examples/p2p-echo/.env.robot
cargo run --locked -p posemesh-p2p-echo --bin posemesh-p2p-echo-robot
~~~

In the compute terminal:

~~~sh
source compute-node-runner-api/examples/p2p-echo/.env.compute
cargo run --locked -p posemesh-p2p-echo --bin posemesh-p2p-echo-compute
~~~

Wait for both to authenticate and poll DMS. The robot first waits for its peer
and relay to become ready. DMS checks worker capability availability when
creating jobs. See [troubleshooting](../reference/configuration.md#troubleshooting)
if a worker cannot become ready.

## Submit the work

In the third terminal, set the same DMS endpoint, organization, and Domain.
Use the Peer IDs recorded above and a [Domain access token for job submission](../how-to/provision-workers.md#authorize-job-submission):

~~~sh
export DMS_BASE_URL='https://DMS_HOST/v1'
export ECHO_ORGANIZATION_ID='<organization UUID>'
export ECHO_DOMAIN_ID='<robot Domain UUID>'
export ECHO_COMPUTE_PEER_ID='<compute Peer ID>'
export ECHO_ROBOT_PEER_ID='<robot Peer ID>'
export APP_JWT_FILE='/absolute/path/to/domain-access-token'
python3 compute-node-runner-api/examples/p2p-echo/scripts/submit.py --dry-run
~~~

The dry run prints the dedicated robot serve job. It reads no token and contacts
no service. Submit the pair:

~~~sh
python3 compute-node-runner-api/examples/p2p-echo/scripts/submit.py \
  --message "hello from Compute to Robot" --timeout-seconds 120
~~~

The helper starts the robot task, waits for its `phase=ready` progress, and
validates its run ID, organization, Domain, protocol, and both Peer IDs. It then
copies the route into a separate compute send job. Each task gets one attempt.

Success means both tasks reach `completed`. The helper prints each task's
ID, status, and progress: the robot reports `phase=echoed` and compute reports
`phase=verified`. Both include the message.

## Stop and inspect

The helper cancels its own jobs on failure or interruption. If cancellation
fails, it prints the job ID: cancel that job through
`POST /v1/jobs/{job_id}/cancel` before stopping the workers.
After both tasks complete, press Ctrl-C in each worker terminal.
The host drains active work and shuts down its peer resources.

Read the [runners](../../core/compute-node-runner-api/examples/p2p-echo/src/lib.rs)
and [submission helper](../../core/compute-node-runner-api/examples/p2p-echo/scripts/submit.py).
The wire protocol is the SDK's `/example/echo/1.0.0`.
The jobs have no completion dependency: the robot must remain active while
compute sends. A dependency on robot completion would prevent that interaction.

Use isolated workers for the two example capabilities. Peer IDs in metadata
check the execution; they do not make DMS schedule on a particular worker.
Continue with [your own runner](../how-to/write-a-runner.md).

## Use direct TCP on one machine

Before starting the robot, replace its relay setting with these values:

~~~sh
export AUKI_P2P_RELAY_MODE=disabled
export AUKI_P2P_LISTEN_MULTIADDRS=/ip4/127.0.0.1/tcp/4001
export AUKI_P2P_ADVERTISED_MULTIADDRS=/ip4/127.0.0.1/tcp/4001
~~~

For different hosts, advertise an address compute can reach. Compute needs
outbound connectivity and does not book its own relay. This changes transport;
both workers still authenticate with DDS and execute DMS tasks.

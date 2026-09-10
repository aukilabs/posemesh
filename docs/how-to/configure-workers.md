# Configure workers

Configure the compute or robot host with its existing worker credentials.
The host handles runtime authentication and renewal. If you need to create a
worker or assign a robot to a Domain first, see [Provision workers](provision-workers.md).

## Choose an environment

Export DDS and DMS endpoints for the same environment:

~~~sh
export DDS_BASE_URL=https://DDS_HOST
export DMS_BASE_URL=https://DMS_HOST/v1
~~~

The DMS base includes `/v1`; the DDS base does not include `/api/v1`.
Check the [deployment requirements](provision-workers.md#check-deployment-support)
for robot, task lease, and P2P support.

## Compute node

Supply the complete compute registration credentials and its wallet key:

~~~sh
export REG_SECRET=COMPUTE_REGISTRATION_CREDENTIALS
export SECP256K1_PRIVHEX=COMPUTE_WALLET_PRIVATE_KEY
~~~

`REG_SECRET` contains the node ID and registration secret encoded together;
[provisioning](provision-workers.md#create-a-compute-node) shows the conversion.
`SECP256K1_PRIVHEX` is the node wallet's 32-byte private key in hex. DDS applies
its wallet registration/staking policy. This key is separate from the P2P key.

The compute entrypoint registers its runner capabilities at startup. Node mode
is configured in DDS: public nodes participate in public scheduling, while
dedicated nodes serve their organization across its Domains.

## Robot

Point the host at the complete, opaque credential returned by DDS:

~~~sh
export ROBOT_REGISTRATION_CREDENTIALS_FILE=/absolute/path/to/robot-credential
~~~

Alternatively, supply `ROBOT_REGISTRATION_CREDENTIALS` inline. Set exactly one
of those two variables. The credential file is read at startup; replacing it
requires a restart.

A robot uses its registration credential without a wallet. It claims dedicated
tasks in its assigned Domain. See [provisioning and assignment](provision-workers.md#create-and-assign-a-robot)
if it has not been assigned yet.

## Enable P2P when needed

For a runner that exchanges data with peers, enable P2P and provide a persistent
identity key:

~~~sh
export AUKI_P2P_ENABLED=true
export AUKI_P2P_PRIVATE_KEY_FILE=/absolute/path/to/worker-peer.key
~~~

Generate a separate key for each worker using the [tutorial's key setup](../tutorials/robot-and-compute.md#prepare-the-workers).
P2P is disabled by default. When enabled, a robot books a relay by default;
compute uses outbound connections during its task. See the
[configuration reference](../reference/configuration.md#p2p) for direct routes,
relay settings, and key requirements.

## Run your worker

Export these values in the terminal that starts the binary, or source your
configured environment file there. The binaries do not load environment files
automatically.

Continue with the [paired Echo tutorial](../tutorials/robot-and-compute.md) or
[your own runner](write-a-runner.md). Echo also needs `ECHO_ORGANIZATION_ID` and
`ECHO_DOMAIN_ID` on both workers; the tutorial's environment templates include
them. See the [configuration reference](../reference/configuration.md) for
all defaults and troubleshooting.

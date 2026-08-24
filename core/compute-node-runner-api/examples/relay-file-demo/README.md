# Dev relay file demo

This example builds two small executables around the real Compute Node engine:

- `posemesh-relay-file-robot` authenticates as a Robot, books a dev relay,
  registers an existing local file, uploads its immutable P2P reference to the
  Domain Server, and creates the reconstruction task.
- `posemesh-relay-file-reconstruction` authenticates as a Compute node, leases
  that task, receives the peer-bound P2P token from DMS, downloads the reference
  artifact, and fetches the file over the exact relay circuit.

There is no synthetic P2P credential or local fake control plane in this flow.
The reference is deliberately circuit-only, and both sides use the production
runner, DDS/DMS, Domain artifact, relay-booking, source-admission, endpoint-auth,
streaming, and SHA-256 verification paths.

## Prepare

From `posemesh/core`:

```sh
cp compute-node-runner-api/examples/relay-file-demo/.env.robot.example /tmp/relay-demo.robot.env
cp compute-node-runner-api/examples/relay-file-demo/.env.reconstruction.example /tmp/relay-demo.reconstruction.env
chmod 600 /tmp/relay-demo.robot.env /tmp/relay-demo.reconstruction.env
```

Fill the two files. The Robot file needs:

- its DDS registration-credential file;
- an App JWT authorized for the selected `DOMAIN_ID`;
- a non-empty `RELAY_DEMO_SOURCE_FILE`.

The reconstruction file needs its existing `REG_SECRET` and
`SECP256K1_PRIVHEX`. With the default `dedicated` reconstruction task, that
credential must belong to the same organization as the App JWT. Set
`RELAY_DEMO_RECONSTRUCTION_TASK_MODE=public` in the Robot file only when the
Compute registration is public.

The demo uses the production seeded capabilities
`/dmtbot/scan-path/v0` and
`/reconstruction/local-refinement-auki-sdk/v0`, because DMS issues a Compute P2P
token only to an actual reconstruction lease. Stop other eligible dev runners
for the same organization/capability during this test so they cannot claim the
demo task first.

Remove outputs from an earlier run—the binaries refuse to overwrite them:

```sh
rm -f /tmp/posemesh-relay-file-demo.reference.json \
      /tmp/posemesh-relay-file-demo.download
```

## Build and run

```sh
cargo build -p posemesh-relay-file-demo --bins
```

Start the reconstruction side first and wait until it is authenticated and
polling DMS:

```sh
./target/debug/posemesh-relay-file-reconstruction /tmp/relay-demo.reconstruction.env
```

In a second terminal, start the Robot:

```sh
./target/debug/posemesh-relay-file-robot /tmp/relay-demo.robot.env
```

The Robot waits for its DMS presence, creates one dedicated Robot task, waits
for a confirmed relay booking, publishes the file reference, and queues the
reconstruction task. The reconstruction process then downloads and verifies
the exact file.

## Verify

The Robot log prints the source Peer ID, circuit route, artifact ID, and
reconstruction job ID. The reconstruction log ends with
`relay demo download verified successfully`.

Inspect the circuit-only reference and compare the files:

```sh
jq '.peer_id, .multiaddrs, .size_bytes, .sha256, .available_until' \
  /tmp/posemesh-relay-file-demo.reference.json
shasum -a 256 /path/from/RELAY_DEMO_SOURCE_FILE \
  /tmp/posemesh-relay-file-demo.download
cmp /path/from/RELAY_DEMO_SOURCE_FILE \
  /tmp/posemesh-relay-file-demo.download
```

During the run, the relay admin endpoint should show one used booking slot.
The reference must contain `relay.dev.aukiverse.com` and `/p2p-circuit/`; no
direct Robot address is accepted by this demo.

Press Ctrl-C once on the Robot to begin the graceful immutable-reference drain.
It can remain alive until `available_until`; press Ctrl-C again to force exit.
Stopping the reconstruction binary requires one Ctrl-C.

Never commit either populated `.env` file, the App JWT, registration credential,
or wallet key.

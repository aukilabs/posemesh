# Manage task lifecycle

Implement these behaviors in each runner that performs ongoing work.
The host delegates DMS heartbeats and reporting to the SDK; your runner controls the work
it starts.

## Progress and results

Call `ctx.ctrl.progress(json_value).await?` with the latest progress.
Each update replaces the previous progress value. Use
`ctx.ctrl.log_event(json_value).await?` for events to include in a heartbeat;
events remain ordered until an acknowledged heartbeat. The queue accepts up to
1024 events and 64 KiB of serialized JSON; exceeding either limit returns an
error. Progress is independently limited to 64 KiB. Exclude credentials.

Upload result artifacts through `ctx.output`. Returning `Ok(())` asks the host
to complete the task; returning an error asks it to report failure.
The host includes uploaded artifact IDs and metadata in its receipt, including
failure receipts with the runner error as the reason. SDK-managed execution
drains in-flight heartbeats and flushes final events before reporting. Failure
reasons and details are limited to 4096 bytes and 64 KiB respectively.
Progress is not an artifact or an arbitrary completion result.

DMS owns retries through the task's attempt policy. A new attempt may repeat
work after an earlier attempt lost its lease. Use task IDs and application
operation IDs to recognize completed side effects. Do not assume exactly-once
execution.

## Cancellation and lease loss

Check `ctx.ctrl.is_cancelled().await` before starting work and at bounded
intervals during long operations. It becomes true when the host receives DMS
cancellation or loses the lease. Stop work and return promptly.

For a long network wait, race the operation against cancellation, as in the
[Echo runners](../../core/compute-node-runner-api/examples/p2p-echo/src/lib.rs).
Use timeouts. Run blocking or CPU-heavy work away from the async executor and
give it a cooperative stop mechanism too; dropping its awaiting future does
not stop an independent thread or subprocess.

A robot runner must also enforce exclusive physical execution and stop its
hardware on cancellation or loss of authority. Keep the robot busy until
that stop completes. The host's serial task loop does not implement hardware
interlocks.

The host skips normal completion after DMS cancellation or lease loss.
Uploaded artifacts or other side effects may already exist; account for them
when retrying or cleaning up.

## Protocol and storage resources

Obtain `AukiProtocolsHandle::get()` inside each task. A retained compute context
from an earlier task is stopped. Await the close of any protocol registration
you own on success, failure, timeout, and cooperative cancellation.

Always run cleanup before propagating the operation's error. Echo stores the
operation result, awaits endpoint closure, then returns the result.
Also design resources for a dropped future or process failure, when code after
an `await` may never execute.

Use input/output ports for ordinary Domain data operations. For a custom
Domain HTTP request, read the current `ctx.access_token.get()` at request time;
a token copied from the initial lease can expire during the task.
The existing synchronous getter reads the SDK-owned rotating token; it returns
an empty string after cancellation, expiry or task completion. Do not log the
token or the full lease. The initial lease snapshot includes metadata from the
first heartbeat and excludes P2P credentials.

## Shut down the host

`run_node` and `run_robot_node` listen for Ctrl-C. Normal shutdown stops
polling, drains active work, and stops authentication and peer resources.
The robot entrypoint handles a second Ctrl-C by interrupting active work.
A compute runner needs its own bounded operations to finish draining.

For an application's existing signal handler, use `run_node_with_shutdown`,
`run_robot_node_with_shutdown`, or the robot variant with separate graceful
and forced tokens. These functions take `tokio_util::sync::CancellationToken`;
await the host future after signalling it. These entrypoints do not install a
SIGTERM handler for your application.

Both entrypoints await SDK registration, authentication, transfer and peer
cleanup, including the compute registrar. The older public registration and
heartbeat helpers remain available to existing callers; the entrypoints no
longer start them. Do not start a second registrar or heartbeat owner alongside
a managed host.

Authentication or registration failure stops the managed host after the SDK's
bounded retries. Robot refresh denial does not fall back to SIWE or loop forever.
Restart only after resolving credentials/assignment or deployment support.

Robot shutdown stops authority renewal, awaits peer shutdown, and releases
relay resources. One live process may own a given P2P private key.
Cancel unfinished demo jobs before stopping workers that are waiting for them.

## Domain transfer compatibility

Existing `Runner`, `TaskCtx`, input/output and control traits are unchanged.
Managed storage keeps CID/Domain URL lookup, temporary `datasets/<scan>/...`
layout, artifact names/types, replacement IDs and receipt metadata. The task's
selected Domain and renewed server URL determine request authority. A URL for
another Domain is rejected. Standalone `DomainClient::new` and the existing
Domain HTTP bindings retain their separate legacy behavior.

Managed buffered uploads require Domain Server `/api/v1/info` upload limits.
Larger files stream through SDK multipart uploads with bounded 16 MiB parts;
the server must advertise multipart support and return upload/data IDs, part
size and expiry. Metadata must follow the current UUID-based Domain contract.
The optional `ArtifactSink::open_multipart` writer interface remains unsupported;
use `put_file` for whole files. Confirm the deployed DDS/DMS/Domain Server
versions before rollout; local fixtures are not proof of deployed support.

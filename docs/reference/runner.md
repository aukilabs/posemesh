# Runner reference

The [runner API](../../core/compute-node-runner-api/src/runner.rs) defines the
ports; [the host](../../core/compute-node/src/engine.rs) supplies their concrete
implementations. Start with [Write a runner](../how-to/write-a-runner.md) for a
complete project.

## Runner and registry

| API | Contract |
| --- | --- |
| `Runner: Send + Sync` | Implements `capability() -> &'static str` and async `run(TaskCtx) -> anyhow::Result<()>` |
| `RunnerRegistry::register(runner)` | Adds a runner; a duplicate capability replaces the previous registration |
| `RunnerRegistry::capabilities()` | Returns the registered strings in sorted order |
| `run_node(config, runners)` | Runs a compute host; separately call `spawn_registration_if_configured` to advertise capabilities |
| `run_robot_node(config, runners)` | Runs a robot host and registers its capabilities |
| `RunnerComposition::with_protocols(builder)` | Injects the host's lazy `AukiProtocolsHandle` while constructing the registry |

Registry and host types are in `posemesh_compute_node::engine`. Configuration
is in `posemesh_compute_node::config`. The runner crate re-exports `Runner`,
`TaskCtx`, `InputSource`, `ArtifactSink`, `ControlPlane`, `MaterializedInput`,
`LeaseEnvelope`, and `TaskSpec` from its root.

## TaskCtx

| Field | What to use it for |
| --- | --- |
| `lease: &LeaseEnvelope` | Current task, Domain, storage endpoint, and lease snapshot |
| `input: &dyn InputSource` | Read task inputs from Domain storage |
| `output: &dyn ArtifactSink` | Upload artifacts and collect completion receipt metadata |
| `ctrl: &dyn ControlPlane` | Observe cancellation and report progress/events |
| `access_token: &dyn AccessTokenProvider` | Read the current renewable Domain bearer with `get()` |

`TaskSpec` includes the capability, task/job IDs, `inputs_cids`, `outputs_prefix`,
opaque JSON `meta`, mode, and attempt information. Many fields in the wire types
are optional; validate fields your runner requires. A custom runner's parameters
belong in `task.meta`. The host requires `lease.domain_id` and
`lease.domain_server_url` even for a runner that does not use artifacts.

The context borrows task-scoped ports. It cannot be retained as process state.
P2P credentials are removed from the runner-visible lease. Use an injected
protocol handle for peer operations, and read `access_token.get()` immediately
before any direct Domain request rather than caching the lease's token.

## InputSource

| Method | Result |
| --- | --- |
| `get_bytes_by_cid(cid)` | Artifact bytes in memory |
| `materialize_cid_to_temp(cid)` | Local temporary file path |
| `materialize_cid_with_meta(cid)` | `MaterializedInput` with a path and optional Domain data ID, name, type, Domain, and related/extracted paths |

Treat materialized paths as temporary task inputs. A CID is the storage
identifier passed by DMS; do not assume it is a filesystem path or a P2P route.

## ArtifactSink

| Method | Use |
| --- | --- |
| `put_bytes(rel_path, bytes)` | Upload bytes with the host's inferred name/type |
| `put_file(rel_path, file_path)` | Upload a local file |
| `put_domain_artifact(request)` | Supply explicit Domain metadata; returns an optional data ID |
| `put_domain_artifact_with_metadata(request, metadata)` | Also attach JSON to this artifact's completion receipt entry |

Paths are relative to the task's `outputs_prefix`; do not prepend it twice.
`DomainArtifactRequest` contains `rel_path`, `name`, `data_type`, `existing_id`,
and `content`, which is `DomainArtifactContent::Bytes` or `::File`. These types
and `AccessTokenProvider` live in `compute_runner_api::runner` when using the
dependency alias from the guide. An explicit `existing_id` selects an update.

The current host's `open_multipart` implementation panics. Use the supported
upload methods above; the presence of that optional trait method does not
indicate host support.

On completion, the host reports `output_cids` from returned artifact IDs and
`meta` containing `job` identity and an `artifacts` array. Each artifact entry
contains `logical_path`, `name`, `data_type`, `id`, and optional `metadata`.
The job details response exposes these under `receipts[].meta.artifacts`,
separate from `tasks[].meta.progress`. Output metadata also accompanies reported
failures. Uploading an artifact does not by itself complete a DMS task.

## ControlPlane

`is_cancelled().await` reads the host's current cancellation flag.
`progress(value).await` replaces the latest JSON progress snapshot.
`log_event(value).await` appends an event for the next heartbeat.
These calls report to the host; successful return is not a DMS delivery receipt.

Returning `Ok(())` lets the host report completion. Returning `Err` lets it
report failure, with DMS deciding retry policy. Both paths need cleanup.
See [task lifecycle](../how-to/task-lifecycle.md) for lease loss, idempotency,
shutdown entrypoints, and long-running work.

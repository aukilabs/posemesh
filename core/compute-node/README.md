# posemesh-compute-node

The shared Rust host for compute nodes and robots. It adapts the existing
`Runner`/`TaskCtx` interfaces to SDK-managed machine authentication, DMS leases,
heartbeats, Domain data transfers and peer lifetimes. Posemesh owns runner
composition, artifact naming/receipts and application execution.

Start with [robot and compute Echo](../../docs/tutorials/robot-and-compute.md),
then [write a runner](../../docs/how-to/write-a-runner.md).
See [worker setup](../../docs/how-to/configure-workers.md),
[task lifecycle](../../docs/how-to/task-lifecycle.md), and the
[configuration reference](../../docs/reference/configuration.md).

Runner, `TaskCtx`, storage and host entrypoints remain compatible. The old
heartbeat/session/authentication helpers and node-registration crate are
removed; registration and shutdown belong to the managed SDK worker.
The [migration notes](../../docs/how-to/task-lifecycle.md#shut-down-the-host)
cover the retained registration shim, health router and removed callback.

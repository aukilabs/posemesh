# posemesh-compute-node-runner-api

The Rust contract for capability implementations: `Runner`, `TaskCtx`, input
and artifact ports, and progress/cancellation control. Task types come from
`auki-dms`; the host supplies the port implementations.

[Write a runner](../../docs/how-to/write-a-runner.md) or consult the
[runner reference](../../docs/reference/runner.md).
For application data exchange, see [P2P in a runner](../../docs/how-to/use-p2p.md).

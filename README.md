# Posemesh

Build compute nodes and robots on the Auki network with a Rust `Runner`.
Posemesh handles machine authentication, DMS task polling, heartbeats, storage
access, and reporting results. Your runner implements the capability.

Use [Auki SDK](https://github.com/aukilabs/auki-sdk) protocols to exchange data
with other peers during a task.

## Start here

[Run a robot and compute node together](docs/tutorials/robot-and-compute.md).
Both claim dedicated DMS tasks, exchange an Echo message over P2P, and report
completion.

You need Rust 1.89+, Python 3, and [configured workers](docs/how-to/configure-workers.md)
in a compatible DDS/DMS environment.

## What are you building?

| Build | Identity | Task placement |
| --- | --- | --- |
| Compute node | DDS registration and a signing wallet; DDS applies staking requirements | Public or dedicated work across eligible Domains |
| Robot | DDS robot credentials, without a compute wallet or stake | Dedicated work in its assigned Domain |

Both use the same runner interface. The entrypoint selects machine authentication.
For user apps and backend services that connect to peers, start with
[Auki SDK](https://github.com/aukilabs/auki-sdk).

## Documentation

| I want to… | Read |
| --- | --- |
| Provision identities and get a job token | [Configure workers](docs/how-to/configure-workers.md) |
| Implement my own task capability | [Write a runner](docs/how-to/write-a-runner.md) |
| Exchange data with another peer | [Use P2P in a runner](docs/how-to/use-p2p.md) |
| Handle cancellation, retries, and shutdown | [Manage task lifecycle](docs/how-to/task-lifecycle.md) |
| Understand the runtime and services | [How tasks execute](docs/explanation/task-execution.md) |
| Look up traits, ports, and results | [Runner reference](docs/reference/runner.md) |
| Look up environment variables or troubleshoot | [Configuration reference](docs/reference/configuration.md) |

## Repository

| Directory | Contents |
| --- | --- |
| [`core/compute-node/`](core/compute-node/) | Shared host for compute nodes and robots |
| [`core/compute-node-runner-api/`](core/compute-node-runner-api/) | Runner interface and the paired Echo example |
| [`core/domain-http/`](core/domain-http/) | Domain HTTP client and bindings |

The Rust workspace is in `core/`. To work on the implementation or other
components in this repository, see [Contributing](CONTRIBUTING.md).

[MIT license](LICENSE). Report security issues privately to security@aukilabs.com.

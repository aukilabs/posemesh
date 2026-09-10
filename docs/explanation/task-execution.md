# How tasks execute

A compute node and a robot run the same Posemesh task engine. Their identity
and task placement differ; the capability implementation uses the same
`Runner` interface.

## Responsibilities

| Component | Owns |
| --- | --- |
| DDS | Worker identities, capability registration, robot Domain assignment, and Domain/P2P authorization |
| DMS | Job submission, scheduling, task state, leases, and terminal results |
| Posemesh host | Machine authentication lifecycle, task polling, execution, heartbeats, and storage ports |
| Your runner | Task input validation, application work, progress, cancellation response, and application resource cleanup |
| Auki SDK | Authenticated peer connections, protocol streams, relay bookings, and peer shutdown |

~~~mermaid
flowchart LR
    App[Job submitter] -->|Submit work| DMS[DMS]
    DMS <-->|Lease, heartbeat, result| Compute[Compute host and runner]
    DMS <-->|Lease, heartbeat, result| Robot[Robot host and runner]
    Compute <-->|Application data through SDK P2P| Robot
~~~

A job contains tasks and may have completion dependencies between them.
DMS assigns work by capability and placement. A capability identifies the work
a runner can execute; a protocol ID identifies the conversation peers use.
Registering one does not register the other.

## Compute nodes and robots

A compute node registers with DDS using registration credentials and wallet
signatures (SIWE). DDS applies its staking policy. Public nodes receive public
work across organizations and Domains, preferring their own organization.
Dedicated nodes work within their organization across its Domains, with
dedicated tasks taking priority over public tasks.

A robot uses an opaque DDS registration credential without a compute wallet
or stake. It can receive dedicated tasks in its assigned Domain. An unassigned
robot cannot claim work. To change its assignment, it must be offline and its
active leases and authorization must drain.

The DMS lease provides access to the task's Domain. Machine registration does
not grant unrestricted Domain access. See the SDK's
[role guide](https://github.com/aukilabs/auki-sdk/blob/main/docs/explanation/apps-nodes-and-robots.md)
for the shared terminology.

## One task's lifecycle

The host polls DMS with its available capabilities. When it obtains a lease,
it initializes task state, sends a heartbeat, builds storage ports, and calls
the matching `Runner::run`. For P2P-enabled compute, it also starts a peer using
the lease's Domain authority.

While the runner executes, the host sends heartbeats and renews Domain/P2P
authority. It forwards progress and events. After the runner returns, the host
reports completion or failure with the uploaded artifacts.

DMS currently allows one active task lease per node, and the host executes
one task at a time. DMS cancellation or lease loss signals the runner to stop;
the runner must respond. See [task lifecycle](../how-to/task-lifecycle.md).

## Two peer lifetimes

Compute creates a peer for a task's Domain when the lease contains P2P
authority. It stops that peer when the task ends. The next task may use another
Domain; obtain the current context inside each `run` call.

A robot has one peer for its assigned Domain for the process lifetime. An Echo
serve task mounts and closes an endpoint while that peer remains alive.
Your application owns each protocol registration's lifetime.

DMS remains the source of truth for work. P2P carries application data, such
as inputs, observations, or streaming results. DDS discovery and DMS relay
booking are separate from task scheduling. The Echo helper exchanges the robot's
route through task progress and metadata; it does not use peer discovery.

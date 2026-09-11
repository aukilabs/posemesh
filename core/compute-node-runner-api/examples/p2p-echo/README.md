# Robot and compute Echo

Two native Rust workers execute dedicated DMS tasks and exchange an Echo
message through Auki SDK P2P.

Follow the [tutorial](../../../../docs/tutorials/robot-and-compute.md).
The [runner implementations](src/lib.rs), two [entrypoints](src/bin/), and
[submission helper](scripts/submit.py) are the source for that guide.

Run `make ci-compute-node` from `posemesh/core` for the local checks.

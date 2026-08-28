# auki-p2p-dataset

`auki-p2p-dataset` is a Posemesh application protocol built on the Auki SDK
peer facade. It
owns the versioned dataset request/response wire, immutable references, file
hashing, relay-capacity filtering, bounded fallback, and atomic downloads.

The crate exports two layers:

- `P2pDatasetAdapter` / `P2pDatasetServer` for a process composition root;
- `DatasetService`, a narrow cloneable facade that dataset-aware runners take
  explicitly in their constructors.

The host constructs the adapter from one fixed-Domain
`AukiPeerProtocolContext`. `AukiPeer` owns the network runtime, current
authority, and route catalog, including relay allocation and recovery. The
dataset protocol sees only authenticated protocol, authorization, and
read-only route views. It selects eligible routes at reference commit and
revalidates the exact catalog revision and authority fences atomically.

This boundary is intentional: dataset code knows nothing about DMS booking IDs
or raw network ownership, and the SDK knows nothing about files, hashes, or
dataset messages.

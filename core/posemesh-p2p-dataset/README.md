# posemesh-p2p-dataset

`posemesh-p2p-dataset` is a Posemesh application protocol built on the Auki SDK
peer facade. It owns the versioned dataset request/response wire, immutable
references, file hashing, relay-capacity filtering, bounded fallback, and
atomic downloads.

The first product-owned contract uses protocol `/posemesh/dataset/1.0.0` and
reference schema `posemesh-dataset/v1`. Each confirmed SDK relay provider is
published as an atomic TCP/WSS pair so the same immutable reference carries a
native and browser-compatible route. The reference keeps the SDK's 16 logical
route-slot limit and can therefore contain at most 19 physical addresses.

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

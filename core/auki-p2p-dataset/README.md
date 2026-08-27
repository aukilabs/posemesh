# auki-p2p-dataset

`auki-p2p-dataset` is the first application protocol built on `auki-p2p`. It
owns the versioned dataset request/response wire, immutable references, file
hashing, relay-capacity filtering, bounded fallback, and atomic downloads.

The crate exports two layers:

- `P2pDatasetAdapter` / `P2pDatasetServer` for a process composition root;
- `DatasetService`, a narrow cloneable facade that dataset-aware runners take
  explicitly in their constructors.

The host constructs the adapter with its shared `Node`, `DomainAuthority`,
and `RouteCatalog`. A DMS relay-booking coordinator publishes and tombstones
generic confirmed routes in that catalog; it never calls into dataset internals.
The dataset protocol selects eligible routes at reference commit and revalidates
the exact catalog revision and authority fences atomically.

This boundary is intentional: dataset code knows nothing about DMS booking IDs,
and relay-booking code knows nothing about files, hashes, or dataset messages.

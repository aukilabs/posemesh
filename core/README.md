# Posemesh Core

Posemesh Core is a list of Rust libraries that implements all of the underlying network code for efficient and optimized communication between nodes in the Posemesh network. The module is designed to simplify the process of running a peer-to-peer (P2P) Posemesh node, allowing easy and seamless P2P communication within the Posemesh network. With this module, nodes can join the network, discover peers, and exchange messages in a decentralized, scalable, and resilient manner.

- [auki-p2p](https://github.com/aukilabs/auki-sdk/tree/main/crates/auki-p2p) – Shared mutually authenticated P2P runtime,
  exact-route transport, relay primitives, and authority-fenced route catalog.
- [auki-p2p-dataset](auki-p2p-dataset/README.md) – Posemesh-owned immutable
  file publication and transfer protocol built on the shared runtime.
- [posemesh-domain-http](domain-http/README.md) – A cross-platform HTTP client library for interacting with posemesh domains on the Auki Network, supporting both native and WebAssembly environments.

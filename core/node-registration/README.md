# Posemesh Node Registration SDK

This crate packages the logic required for nodes to register with the discovery service.

## Modules

- `crypto`: helpers for secp256k1 key loading, signature generation, and timestamp formatting.
- `http`: an `axum` router that handles DDS callbacks (e.g. `/internal/v1/registrations`) and the DDS health probe.
- `persist`: in-memory storage used to cache the most recent node secret returned by DDS callbacks.
- `state`: persistence primitives for registration status, last DDS health check, and a file-based advisory lock.
- `register`: async registration client that periodically signs and submits node metadata to DDS.

## Adding the Dependency

```toml
# Cargo.toml
[dependencies]
posemesh-node-registration = "0.1.0"
```

## Example: Mounting the HTTP Routes

```rust
use axum::Router;
use posemesh_node_registration::http::{router_dds, DdsState};
use std::path::PathBuf;

fn build_app() -> Router {
    let dds_state = DdsState {
        secret_path: PathBuf::from("data/node_secret"),
    };

    // Merge with your existing routes as needed
    router_dds(dds_state)
}
```

The callback handler automatically persists secrets in-memory via `persist::write_node_secret_to_path`. 
Consumers that need to read the cached secret can call `persist::read_node_secret()`.

## Example: Spawning the Registration Loop

```rust
use posemesh_node_registration::register::{self, RegistrationConfig};

async fn spawn_registration(config: RegistrationConfig) {
    tokio::spawn(register::run_registration_loop(config));
}

fn build_registration_config() -> RegistrationConfig {
    RegistrationConfig {
        dds_base_url: "https://dds.auki.network".into(),
        node_url: "https://node.example.com".into(),
        node_version: "1.0.0".into(),
        reg_secret: "my-reg-secret".into(),
        secp256k1_privhex: std::env::var("SECP256K1_PRIVHEX").expect("missing key"),
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("client"),
        register_interval_secs: 60,
        max_retry: -1,
        capabilities: vec![
            "/reconstruction/global-refinement/v1".into(),
            "/reconstruction/local-refinement/v1".into(),
        ],
    }
}
```

`run_registration_loop` takes care of:

- Deriving the secp256k1 public key and signatures for registration payloads.
- Persisting registration state and last health-check timestamps.
- Enforcing a cross-process file lock so that only one registrar runs at a time.
- Exponential backoff with jitter when DDS requests fail.

# posemesh-domain-http

This package provides an HTTP client library for interacting with posemesh domains on the Auki Network, supporting both native and WebAssembly targets.

Key features:
- Authenticate as an app or user to obtain access tokens for domain operations.
- Download domain data with streaming support for efficient handling of large datasets.
- Upload domain data, supporting both creation and update operations.
- Cross-platform support: works on native platforms (using `tokio` and `reqwest`) and in the browser via WASM.

## Auki Console
Create an account on https://console.auki.network, and create an app for your application if you only need read access.

## Javascript
```
let domainClient = await signInWithAppCredential("https://api.auki.network", "https://dds.auki.network", ${app_key}, ${app_key}, ${app_secret});

// download domain data
await domainClient.downloadDomainData(domainId, query, function (data) {
    // Example of domain data:
    // {
    //   id: "12345",
    //   domain_id: "my-domain-id",
    //   name: "example.csv",
    //   data_type: "dmt_accel_csv",
    //   size: 1024,
    //   created_at: "2024-06-01T12:00:00Z",
    //   updated_at: "2024-06-01T12:00:00Z",
    // }

    // Data binary
    const dataBytes = data.get_data_bytes();
});

// download metadata only
const metadataList = await domainClient.downloadDomainDataMetadata(domainId, query);
```

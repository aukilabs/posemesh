use httpmock::prelude::*;
use posemesh_compute_node::storage::{
    client::{DomainClient, UploadRequest},
    TokenRef,
};

#[tokio::test]
async fn token_rotation_applies_to_subsequent_requests() {
    let server = MockServer::start();

    // Initial token A
    let token = TokenRef::new("tA".into());
    let base: url::Url = server.base_url().parse().unwrap();
    let client = DomainClient::new(base, token.clone()).unwrap();

    // First request should use Bearer tA
    let boundary = "BOUNDARY";
    let body = format!(
        "--{boundary}\r\n\
Content-Type: application/octet-stream\r\n\
Content-Disposition: form-data; name=\"scan\"; data-type=\"refined_scan\"; id=\"c1\"; domain-id=\"dom1\"\r\n\r\n\
payload\r\n\
--{boundary}--\r\n"
    );
    let m1 = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v1/domains/dom1/data/c1")
            .header("authorization", "Bearer tA")
            .header("accept", "multipart/form-data");
        then.status(200)
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(body.clone());
    });
    let uri1 = format!("{}/api/v1/domains/dom1/data/c1", server.base_url());
    let _ = client.download_uri(&uri1).await.unwrap();
    m1.assert();

    // Rotate token to B
    token.swap("tB".into());

    // Next request should use Bearer tB (GET)
    let body2 = format!(
        "--{boundary}\r\n\
Content-Type: application/octet-stream\r\n\
Content-Disposition: form-data; name=\"scan\"; data-type=\"refined_scan\"; id=\"c2\"; domain-id=\"dom1\"\r\n\r\n\
payload\r\n\
--{boundary}--\r\n"
    );
    let m2 = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v1/domains/dom1/data/c2")
            .header("authorization", "Bearer tB")
            .header("accept", "multipart/form-data");
        then.status(200)
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(body2.clone());
    });
    let uri2 = format!("{}/api/v1/domains/dom1/data/c2", server.base_url());
    let _ = client.download_uri(&uri2).await.unwrap();
    m2.assert();

    // And an upload should also use Bearer tB
    let m3 = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v1/domains/dom1/data")
            .header("authorization", "Bearer tB")
            .body_contains("file_bin");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"data":[{"id":"data-1"}]}"#);
    });
    client
        .upload_artifact(UploadRequest {
            domain_id: "dom1",
            name: "file_bin",
            data_type: "binary",
            logical_path: "out/file.bin",
            bytes: b"bin",
            existing_id: None,
        })
        .await
        .unwrap();
    m3.assert();
}

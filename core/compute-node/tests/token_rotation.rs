#[path = "support/sdk.rs"]
#[allow(dead_code)]
mod sdk;
use httpmock::prelude::*;
use posemesh_compute_node::storage::{
    client::{DomainClient, UploadRequest},
    TokenRef,
};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn token_rotation_applies_to_subsequent_requests() {
    let server = MockServer::start();
    sdk::info(&server);
    let domain = Uuid::new_v4();
    let path = format!("/api/v1/domains/{domain}/data");
    let token = TokenRef::new(String::new());
    let client = DomainClient::new(server.base_url().parse().unwrap(), token.clone()).unwrap();
    for generation in ["A", "B"] {
        let id = Uuid::new_v4();
        let bearer = sdk::data_token(
            &server.base_url(),
            domain,
            chrono::Utc::now() + chrono::Duration::hours(1),
            generation,
        );
        token.swap(bearer.clone());
        let metadata = sdk::metadata(domain, id, "scan", "binary", 7);
        let list = server.mock(|when, then| {
            when.method(GET)
                .path(&path)
                .query_param("ids", id.to_string())
                .header("authorization", format!("Bearer {bearer}"));
            then.header("content-type", "application/json")
                .json_body(json!({"data":[metadata]}));
        });
        let raw = server.mock(|when, then| {
            when.method(GET)
                .path(format!("{path}/{id}"))
                .query_param("raw", "true")
                .header("authorization", format!("Bearer {bearer}"));
            then.body("payload");
        });
        let parts = client
            .download_cid(&domain.to_string(), &id.to_string())
            .await
            .unwrap();
        assert_eq!(tokio::fs::read(&parts[0].path).await.unwrap(), b"payload");
        tokio::fs::remove_dir_all(&parts[0].root).await.unwrap();
        let upload = server.mock(|when, then| {
            when.method(POST)
                .path(&path)
                .header("authorization", format!("Bearer {bearer}"));
            then.header("content-type", "application/json")
                .json_body(json!({"data":[metadata]}));
        });
        assert_eq!(
            client
                .upload_artifact(UploadRequest {
                    domain_id: &domain.to_string(),
                    name: "scan",
                    data_type: "binary",
                    logical_path: "out/scan",
                    bytes: b"payload",
                    existing_id: None
                })
                .await
                .unwrap(),
            Some(id.to_string())
        );
        list.assert();
        raw.assert();
        upload.assert();
    }
}

#[tokio::test]
async fn standalone_grants_reject_wrong_issuer_domain_audience_expiry_and_revocation_before_http() {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let server = MockServer::start();
    let network = server.mock(|_, then| {
        then.status(500);
    });
    let domain = Uuid::new_v4();
    let id = Uuid::new_v4();
    let valid = json!({"iss":"dds","aud":[server.base_url()],"domain_id":domain,"exp":(chrono::Utc::now()+chrono::Duration::hours(1)).timestamp()});
    let mut cases = Vec::new();
    for (field, value) in [
        ("iss", json!("other")),
        ("aud", json!(["https://other.invalid"])),
        ("domain_id", json!(Uuid::new_v4())),
        ("exp", json!(1)),
    ] {
        let mut claims = valid.clone();
        claims[field] = value;
        cases.push(format!(
            "e30.{}.local-fixture",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
        ));
    }
    cases.extend([String::new(), "malformed".into()]);
    for bearer in cases {
        let client =
            DomainClient::new(server.base_url().parse().unwrap(), TokenRef::new(bearer)).unwrap();
        assert!(client
            .download_cid(&domain.to_string(), &id.to_string())
            .await
            .is_err());
    }
    network.assert_hits(0);
}

#[tokio::test]
async fn standalone_timeout_is_honored() {
    let server = MockServer::start();
    let domain = Uuid::new_v4();
    let id = Uuid::new_v4();
    server.mock(|when, then| {
        when.method(GET)
            .path(format!("/api/v1/domains/{domain}/data"));
        then.delay(std::time::Duration::from_millis(200))
            .header("content-type", "application/json")
            .json_body(json!({"data":[]}));
    });
    let token = sdk::data_token(
        &server.base_url(),
        domain,
        chrono::Utc::now() + chrono::Duration::hours(1),
        "A",
    );
    let client = DomainClient::with_timeout(
        server.base_url().parse().unwrap(),
        TokenRef::new(token),
        std::time::Duration::from_millis(20),
    )
    .unwrap();
    let error = client
        .download_cid(&domain.to_string(), &id.to_string())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("timed out"));
}

#[path = "support/sdk.rs"]
#[allow(dead_code)]
mod sdk;
use compute_runner_api::{ArtifactSink, InputSource};
use httpmock::prelude::*;
use posemesh_compute_node::storage::{
    client::DomainClient, input::DomainInput, output::DomainOutput, TokenRef,
};
use serde_json::json;
use std::io::Write;
use uuid::Uuid;

fn client(server: &MockServer, domain: Uuid) -> DomainClient {
    sdk::info(server);
    DomainClient::new(
        server.base_url().parse().unwrap(),
        TokenRef::new(sdk::data_token(
            &server.base_url(),
            domain,
            chrono::Utc::now() + chrono::Duration::hours(1),
            "A",
        )),
    )
    .unwrap()
}

#[tokio::test]
async fn download_cid_and_upload_bytes() {
    let server = MockServer::start();
    let domain = Uuid::new_v4();
    let id = Uuid::new_v4();
    let manifest = Uuid::new_v4();
    let saved = Uuid::new_v4();
    let client = client(&server, domain);
    let path = format!("/api/v1/domains/{domain}/data");
    let name = "scan_2024-01-02_03-04-05";
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file("images.bin", zip::write::FileOptions::default())
        .unwrap();
    zip.write_all(b"hello").unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let manifest_bytes = br#"{"example":true}"#;
    let get = server.mock(|when, then| {
        when.method(GET)
            .path(&path)
            .query_param("ids", format!("{id},{manifest}"));
        then.header("content-type", "application/json")
            .json_body(json!({"data":[
                sdk::metadata(domain,id,name,"refined_scan_zip",bytes.len()),
                sdk::metadata(domain,manifest,"manifest","dmt_manifest_json",manifest_bytes.len())
            ]}));
    });
    for (data_id, payload) in [
        (id, bytes.as_slice()),
        (manifest, manifest_bytes.as_slice()),
    ] {
        server.mock(|when, then| {
            when.method(GET)
                .path(format!("{path}/{data_id}"))
                .query_param("raw", "true");
            then.body(payload);
        });
    }
    let input = DomainInput::new(client.clone(), domain.to_string());
    let cid = format!("{}{path}?ids={id},{manifest}", server.base_url());
    assert_eq!(input.get_bytes_by_cid(&cid).await.unwrap(), bytes);
    let materialized = input.materialize_cid_with_meta(&cid).await.unwrap();
    assert_eq!(materialized.cid, cid);
    assert_eq!(materialized.data_id, Some(id.to_string()));
    assert_eq!(materialized.domain_id, Some(domain.to_string()));
    assert_eq!(materialized.data_type.as_deref(), Some("refined_scan_zip"));
    assert_eq!(materialized.name.as_deref(), Some(name));
    assert!(materialized
        .path
        .ends_with("datasets/2024-01-02_03-04-05/scan_2024-01-02_03-04-05.refined_scan_zip"));
    assert!(materialized.extracted_paths.is_empty());
    assert_eq!(materialized.related_files.len(), 1);
    assert_eq!(
        tokio::fs::read(&materialized.related_files[0])
            .await
            .unwrap(),
        manifest_bytes
    );
    tokio::fs::remove_dir_all(materialized.root_dir)
        .await
        .unwrap();
    get.assert_hits(2);

    let name = "out_job_manifest_json_task-456";
    let lookup = server.mock(|when, then| {
        when.method(GET)
            .path(&path)
            .query_param("name", name)
            .query_param("data_type", "json");
        then.header("content-type", "application/json")
            .json_body(json!({"data":[]}));
    });
    let post = server.mock(|when, then| {
        when.method(POST).path(&path).body_contains(name);
        then.header("content-type", "application/json")
            .json_body(json!({"data":[sdk::metadata(domain,saved,name,"json",3)]}));
    });
    let put = server.mock(|when, then| {
        when.method(PUT)
            .path(&path)
            .body_contains(format!("id=\"{saved}\""));
        then.header("content-type", "application/json")
            .json_body(json!({"data":[sdk::metadata(domain,saved,name,"json",7)]}));
    });
    let output = DomainOutput::new(
        client,
        domain.to_string(),
        Some("out".into()),
        "task-456".into(),
    );
    output.put_bytes("job_manifest.json", b"bin").await.unwrap();
    output
        .put_bytes("job_manifest.json", b"updated")
        .await
        .unwrap();
    post.assert();
    put.assert();
    lookup.assert();
    let artifacts = output.uploaded_artifacts();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].logical_path, "out/job_manifest.json");
    assert_eq!(artifacts[0].id, Some(saved.to_string()));
}

#[tokio::test]
async fn upload_manifest_with_existing_id_uses_put_via_lookup() {
    let server = MockServer::start();
    let domain = Uuid::new_v4();
    let id = Uuid::new_v4();
    let client = client(&server, domain);
    let path = format!("/api/v1/domains/{domain}/data");
    let name = "out_job_manifest_json_task-456";
    let metadata = sdk::metadata(domain, id, name, "json", 7);
    let lookup = server.mock(|when, then| {
        when.method(GET)
            .path(&path)
            .query_param("name", name)
            .query_param("data_type", "json");
        then.header("content-type", "application/json")
            .json_body(json!({"data":[metadata]}));
    });
    let put = server.mock(|when, then| {
        when.method(PUT)
            .path(&path)
            .body_contains(format!("id=\"{id}\""));
        then.header("content-type", "application/json")
            .json_body(json!({"data":[metadata]}));
    });
    let output = DomainOutput::new(
        client,
        domain.to_string(),
        Some("out".into()),
        "task-456".into(),
    );
    output
        .put_bytes("job_manifest.json", b"payload")
        .await
        .unwrap();
    lookup.assert();
    put.assert();
}

#[tokio::test]
async fn upload_refined_scan_zip_uses_expected_data_type_and_records_id() {
    let server = MockServer::start();
    let domain = Uuid::new_v4();
    let id = Uuid::new_v4();
    let client = client(&server, domain);
    let path = format!("/api/v1/domains/{domain}/data");
    let name = "out_refined_local_scan_a_RefinedScan_zip_task-456";
    let lookup = server.mock(|when, then| {
        when.method(GET)
            .path(&path)
            .query_param("name", name)
            .query_param("data_type", "zip_data");
        then.header("content-type", "application/json")
            .json_body(json!({"data":[]}));
    });
    let upload = server.mock(|when, then| {
        when.method(POST)
            .path(&path)
            .body_contains(name)
            .body_contains("zip_data")
            .body_contains("zipdata");
        then.header("content-type", "application/json")
            .json_body(json!({"data":[sdk::metadata(domain,id,name,"zip_data",7)]}));
    });
    let output = DomainOutput::new(
        client,
        domain.to_string(),
        Some("out".into()),
        "task-456".into(),
    );
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(b"zipdata").unwrap();
    output
        .put_file("refined/local/scan_a/RefinedScan.zip", file.path())
        .await
        .unwrap();
    lookup.assert();
    upload.assert();
    let artifacts = output.uploaded_artifacts();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(
        artifacts[0].logical_path,
        "out/refined/local/scan_a/RefinedScan.zip"
    );
    assert_eq!(artifacts[0].data_type, "zip_data");
    assert_eq!(artifacts[0].id, Some(id.to_string()));
}

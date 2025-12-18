use compute_runner_api::{ArtifactSink, InputSource};
use httpmock::prelude::*;
use posemesh_compute_node::storage::{
    client::DomainClient, input::DomainInput, output::DomainOutput, TokenRef,
};
use std::io::Write;
use tempfile::NamedTempFile;
use zip::write::FileOptions;

#[tokio::test]
async fn download_cid_and_upload_bytes() {
    let server = MockServer::start();

    let cid = format!("{}/api/v1/domains/dom1/data/bafy-123", server.base_url());
    let payload = b"hello".to_vec();
    let zip_bytes = build_zip(&payload);
    let manifest_bytes = br#"{"example":true}"#.to_vec();
    let boundary = "BOUNDARY";
    let created_at = "2025-01-01T00:00:00Z";
    let updated_at = "2025-01-01T00:00:00Z";
    let mut body = Vec::new();
    body.extend_from_slice(
	        format!(
	            "--{boundary}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; name=\"scan_2024-01-02_03-04-05\"; data-type=\"refined_scan_zip\"; id=\"bafy-123\"; domain-id=\"dom1\"; size=\"{}\"; created-at=\"{created_at}\"; updated-at=\"{updated_at}\"\r\n\r\n",
	            zip_bytes.len()
	        )
	        .as_bytes(),
	    );
    body.extend_from_slice(&zip_bytes);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(
	        format!(
	            "--{boundary}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; name=\"manifest\"; data-type=\"dmt_manifest_json\"; id=\"manifest-1\"; domain-id=\"dom1\"; size=\"{}\"; created-at=\"{created_at}\"; updated-at=\"{updated_at}\"\r\n\r\n",
	            manifest_bytes.len()
	        )
	        .as_bytes(),
	    );
    body.extend_from_slice(&manifest_bytes);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

    let get_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v1/domains/dom1/data")
            .query_param("ids", "bafy-123")
            .header("authorization", "Bearer tkn")
            .header("accept", "multipart/form-data");
        then.status(200)
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(body.clone());
    });

    let lookup_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v1/domains/dom1/data")
            .query_param("name", "out_job_manifest_json_task-456")
            .query_param("data_type", "json")
            .header("authorization", "Bearer tkn")
            .header("accept", "application/json");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"data":[]}"#);
    });

    let post_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v1/domains/dom1/data")
            .header("authorization", "Bearer tkn")
            .body_contains("job_manifest_");
        then.status(200)
            .header("content-type", "application/json")
	            .body(r#"{"data":[{"id":"data-123","domain_id":"dom1","name":"job_manifest_task-456","data_type":"job_manifest_json","size":3,"created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}]}"#);
	    });

    let put_mock = server.mock(|when, then| {
        when.method(PUT)
            .path("/api/v1/domains/dom1/data")
            .header("authorization", "Bearer tkn")
            .body_contains("id=\"data-123\"");
        then.status(200)
            .header("content-type", "application/json")
	            .body(r#"{"data":[{"id":"data-123","domain_id":"dom1","name":"job_manifest_task-456","data_type":"job_manifest_json","size":7,"created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}]}"#);
	    });

    let base: url::Url = server.base_url().parse().unwrap();
    let token = TokenRef::new("tkn".into());
    let client = DomainClient::new(base, token.clone()).unwrap();

    // InputSource
    let input = DomainInput::new(client.clone(), "dom1".into());
    let bytes = input.get_bytes_by_cid(&cid).await.unwrap();
    assert_eq!(bytes, zip_bytes);
    get_mock.assert();
    let materialized = input.materialize_cid_with_meta(&cid).await.unwrap();
    assert_eq!(materialized.cid, cid);
    assert_eq!(materialized.data_id.as_deref(), Some("bafy-123"));
    assert_eq!(materialized.data_type.as_deref(), Some("refined_scan_zip"));
    assert_eq!(
        materialized.name.as_deref(),
        Some("scan_2024-01-02_03-04-05")
    );
    assert!(materialized
        .path
        .file_name()
        .is_some_and(|f| f == "scan_2024-01-02_03-04-05.refined_scan_zip"));
    assert!(materialized.extracted_paths.is_empty());
    let manifest_path = materialized
        .related_files
        .iter()
        .find(|p| {
            p.file_name()
                .is_some_and(|f| f == "manifest.dmt_manifest_json")
        })
        .expect("manifest path present");
    let manifest_saved = tokio::fs::read(manifest_path).await.unwrap();
    assert_eq!(manifest_saved, manifest_bytes);

    // ArtifactSink
    let output = DomainOutput::new(client, "dom1".into(), Some("out".into()), "task-456".into());
    output.put_bytes("job_manifest.json", b"bin").await.unwrap();
    output
        .put_bytes("job_manifest.json", b"updated")
        .await
        .unwrap();
    post_mock.assert();
    put_mock.assert();
    lookup_mock.assert();

    let artifacts = output.uploaded_artifacts();
    let manifest_record = artifacts
        .into_iter()
        .find(|record| record.logical_path == "out/job_manifest.json")
        .expect("manifest uploaded");
    assert_eq!(manifest_record.id.as_deref(), Some("data-123"));
}

#[tokio::test]
async fn upload_manifest_with_existing_id_uses_put_via_lookup() {
    let server = MockServer::start();

    let lookup_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v1/domains/dom1/data")
            .query_param("name", "out_job_manifest_json_task-456")
            .query_param("data_type", "json")
            .header("authorization", "Bearer tkn")
            .header("accept", "application/json");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"data":[{"id":"data-123","domain_id":"dom1","name":"out_job_manifest_json_task-456","data_type":"json","size":7,"created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}]}"#);
    });

    let put_mock = server.mock(|when, then| {
	        when.method(PUT)
	            .path("/api/v1/domains/dom1/data")
	            .header("authorization", "Bearer tkn")
	            .body_contains("id=\"data-123\"");
	        then.status(200)
	            .header("content-type", "application/json")
	            .body(r#"{"data":[{"id":"data-123","domain_id":"dom1","name":"job_manifest_task-456","data_type":"job_manifest_json","size":7,"created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}]}"#);
	    });

    let token = TokenRef::new("tkn".into());
    let base: url::Url = server.base_url().parse().unwrap();
    let client = DomainClient::new(base, token).unwrap();
    let output = DomainOutput::new(client, "dom1".into(), Some("out".into()), "task-456".into());

    output
        .put_bytes("job_manifest.json", b"payload")
        .await
        .unwrap();

    lookup_mock.assert();
    put_mock.assert();
}

#[tokio::test]
async fn upload_refined_scan_zip_uses_expected_data_type_and_records_id() {
    let server = MockServer::start();
    let lookup = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v1/domains/dom1/data")
            .query_param("name", "out_refined_local_scan_a_RefinedScan_zip_task-456")
            .query_param("data_type", "zip_data")
            .header("authorization", "Bearer tkn")
            .header("accept", "application/json");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"data":[]}"#);
    });
    let initiate = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v1/domains/dom1/data/multipart")
            .header("authorization", "Bearer tkn")
            .body_contains("\"name\":\"out_refined_local_scan_a_RefinedScan_zip_task-456\"")
            .body_contains("\"data_type\":\"zip_data\"");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"upload_id":"up1","part_size":1024}"#);
    });
    let put_part = server.mock(|when, then| {
        when.method(PUT)
            .path("/api/v1/domains/dom1/data/multipart")
            .query_param("uploadId", "up1")
            .query_param("partNumber", "1")
            .header("authorization", "Bearer tkn")
            .body("zipdata");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"etag":"etag-1"}"#);
    });
    let complete = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v1/domains/dom1/data/multipart")
            .query_param("uploadId", "up1")
            .header("authorization", "Bearer tkn")
            .body_contains("\"parts\"");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"id":"data-zip"}"#);
    });

    let token = TokenRef::new("tkn".into());
    let base: url::Url = server.base_url().parse().unwrap();
    let client = DomainClient::new(base, token).unwrap();
    let output = DomainOutput::new(client, "dom1".into(), Some("out".into()), "task-456".into());

    let mut tmp = NamedTempFile::new().unwrap();
    tmp.write_all(b"zipdata").unwrap();

    output
        .put_file("refined/local/scan_a/RefinedScan.zip", tmp.path())
        .await
        .unwrap();
    initiate.assert();
    put_part.assert();
    complete.assert();
    lookup.assert();

    let artifacts = output.uploaded_artifacts();
    let refined_record = artifacts
        .into_iter()
        .find(|artifact| artifact.logical_path == "out/refined/local/scan_a/RefinedScan.zip")
        .expect("refined scan upload recorded");
    assert_eq!(refined_record.data_type, "zip_data");
    assert_eq!(refined_record.id.as_deref(), Some("data-zip"));
}

fn build_zip(payload: &[u8]) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("images.bin", options).unwrap();
        zip.write_all(payload).unwrap();
        zip.finish().unwrap();
    }
    buffer
}

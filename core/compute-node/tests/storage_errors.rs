#[path = "support/sdk.rs"]
#[allow(dead_code)]
mod sdk;
use uuid::Uuid;

use httpmock::prelude::*;
use posemesh_compute_node::errors::StorageError;
use posemesh_compute_node::storage::{
    client::{DomainClient, UploadRequest},
    TokenRef,
};

#[tokio::test]
async fn download_error_mapping() {
    let server = MockServer::start();
    let domain = Uuid::new_v4().to_string();
    sdk::info(&server);
    let token = sdk::data_token(
        &server.base_url(),
        domain.parse().unwrap(),
        chrono::Utc::now() + chrono::Duration::hours(1),
        "A",
    );
    let statuses = [400, 401, 404, 409, 500];
    for status in statuses {
        let cid = Uuid::new_v4().to_string();
        let cid_for_mock = cid.clone();
        let m = server.mock(|when, then| {
            when.method(GET)
                .path(format!("/api/v1/domains/{domain}/data"))
                .query_param("ids", cid_for_mock.as_str());
            then.status(status);
        });
        let base: url::Url = server.base_url().parse().unwrap();
        let client = DomainClient::new(base.clone(), TokenRef::new(token.clone())).unwrap();
        let uri = base
            .join(&format!("api/v1/domains/{domain}/data/{}", cid))
            .unwrap()
            .to_string();
        let err = client.download_uri(&uri).await.unwrap_err();
        match status {
            400 => assert!(
                matches!(err, StorageError::BadRequest),
                "expected BadRequest, got {:?}",
                err
            ),
            401 => match err {
                StorageError::Unauthorized => {}
                other => panic!("unexpected 401 mapping: {:?}", other),
            },
            404 => assert!(
                matches!(err, StorageError::NotFound),
                "expected NotFound, got {:?}",
                err
            ),
            409 => assert!(
                matches!(err, StorageError::Conflict),
                "expected Conflict, got {:?}",
                err
            ),
            500 => assert!(
                matches!(err, StorageError::Server(500)),
                "expected Server(500), got {:?}",
                err
            ),
            _ => unreachable!(),
        }
        m.assert();
    }
}

#[tokio::test]
async fn download_empty_metadata_maps_to_not_found() {
    let server = MockServer::start();
    let domain = Uuid::new_v4();
    let id = Uuid::new_v4();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/api/v1/domains/{domain}/data"))
            .query_param("ids", id.to_string());
        then.header("content-type", "application/json")
            .json_body(serde_json::json!({"data":[]}));
    });
    let token = sdk::data_token(
        &server.base_url(),
        domain,
        chrono::Utc::now() + chrono::Duration::hours(1),
        "A",
    );
    let client =
        DomainClient::new(server.base_url().parse().unwrap(), TokenRef::new(token)).unwrap();
    assert!(matches!(
        client
            .download_cid(&domain.to_string(), &id.to_string())
            .await,
        Err(StorageError::NotFound)
    ));
    m.assert();
}

#[tokio::test]
async fn upload_error_mapping() {
    let server = MockServer::start();
    let domain = Uuid::new_v4().to_string();
    sdk::info(&server);
    let token = sdk::data_token(
        &server.base_url(),
        domain.parse().unwrap(),
        chrono::Utc::now() + chrono::Duration::hours(1),
        "A",
    );
    let statuses = [400, 401, 404, 409, 500];
    for status in statuses {
        let name = format!("f{}", status);
        let marker = name.clone();
        let m = server.mock(|when, then| {
            when.method(POST)
                .path(format!("/api/v1/domains/{domain}/data"))
                .body_contains(marker.as_str());
            then.status(status);
        });
        let base: url::Url = server.base_url().parse().unwrap();
        let client = DomainClient::new(base.clone(), TokenRef::new(token.clone())).unwrap();
        let logical = format!("out/{}", name);
        let err = client
            .upload_artifact(UploadRequest {
                domain_id: &domain,
                name: &name,
                data_type: "binary",
                logical_path: &logical,
                bytes: b"x",
                existing_id: None,
            })
            .await
            .unwrap_err();
        match status {
            400 => assert!(matches!(err, StorageError::BadRequest)),
            401 => match err {
                StorageError::Unauthorized => {}
                other => panic!("unexpected 401 mapping: {:?}", other),
            },
            404 => assert!(matches!(err, StorageError::NotFound)),
            409 => assert!(matches!(err, StorageError::Conflict)),
            500 => assert!(matches!(err, StorageError::Server(500))),
            _ => unreachable!(),
        }
        m.assert();
    }
}

use base64::{Engine as _, engine::general_purpose};
use posemesh_domain_http::domain_client::DomainClient;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn make_jwt(claims: serde_json::Value) -> String {
    let header = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string());
    format!("{}.{}.sig", header, payload)
}

fn sample_pose(domain_id: &str, lighthouse_id: &str, short_id: &str) -> serde_json::Value {
    json!({
        "id": lighthouse_id,
        "short_id": short_id,
        "domain_id": domain_id,
        "reported_size": 0.16,
        "px": 1.0,
        "py": 2.0,
        "pz": 3.0,
        "rx": 0.0,
        "ry": 0.0,
        "rz": 0.0,
        "rw": 1.0,
        "latitude": null,
        "longitude": null,
        "altitude": null,
        "vertical_accuracy": null,
        "horizontal_accuracy": null,
        "gps_timestamp": null,
        "scanner_device_id": "scanner-1",
        "scanner_device_name": "scanner",
        "scanner_device_model": "model",
        "placed_at": "2026-08-13T00:00:00Z"
    })
}

#[tokio::test]
async fn robot_verify_domain_token_and_list_poses() {
    let dds = MockServer::start().await;
    let ds = MockServer::start().await;

    let robot_id = "11111111-1111-1111-1111-111111111111";
    let domain_id = "22222222-2222-2222-2222-222222222222";
    let lighthouse_id = "33333333-3333-3333-3333-333333333333";
    let short_id = "SHORTID0001";
    let exp = now_unix_secs() + 3600;

    let robot_jwt = make_jwt(json!({
        "exp": exp,
        "sub": robot_id,
        "node_id": robot_id,
        "organization_id": "44444444-4444-4444-4444-444444444444",
        "node_type": "robot",
        "node_mode": "dedicated",
        "assigned_domain_id": domain_id
    }));
    let domain_jwt = make_jwt(json!({
        "exp": exp,
        "sub": domain_id
    }));

    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/robot/verify"))
        .and(body_partial_json(json!({
            "registration_credentials": "robot-secret"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "robot_id": robot_id,
            "access_token": robot_jwt,
            "access_expires_at": "2026-08-13T12:00:00Z"
        })))
        .expect(1..)
        .mount(&dds)
        .await;

    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/robot/domain-token"))
        .and(header("authorization", format!("Bearer {}", robot_jwt)))
        .and(body_partial_json(json!({ "domain_id": domain_id })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "domain_id": domain_id,
            "domain_server_url": ds.uri(),
            "access_token": domain_jwt,
            "access_expires_at": "2026-08-13T12:00:00Z"
        })))
        .expect(1..)
        .mount(&dds)
        .await;

    let pose = sample_pose(domain_id, lighthouse_id, short_id);
    Mock::given(method("GET"))
        .and(path(format!("/api/v1/domains/{}/lighthouses", domain_id)))
        .and(header("authorization", format!("Bearer {}", domain_jwt)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "poses": [pose.clone()]
        })))
        .expect(1)
        .mount(&ds)
        .await;

    Mock::given(method("GET"))
        .and(path(format!(
            "/api/v1/domains/{}/lighthouses/{}",
            domain_id, lighthouse_id
        )))
        .and(header("authorization", format!("Bearer {}", domain_jwt)))
        .respond_with(ResponseTemplate::new(200).set_body_json(pose.clone()))
        .expect(1)
        .mount(&ds)
        .await;

    Mock::given(method("GET"))
        .and(path(format!(
            "/api/v1/domains/{}/lighthouses/{}",
            domain_id, short_id
        )))
        .and(header("authorization", format!("Bearer {}", domain_jwt)))
        .respond_with(ResponseTemplate::new(200).set_body_json(pose.clone()))
        .expect(1)
        .mount(&ds)
        .await;

    let client = DomainClient::new_with_robot_credential(&dds.uri(), "robot-client", "robot-secret")
        .await
        .expect("robot sign-in");

    assert_eq!(client.robot_id().await.as_deref(), Some(robot_id));
    assert_eq!(
        client.assigned_domain_id().await.as_deref(),
        Some(domain_id)
    );

    let poses = client.list_poses(domain_id).await.expect("list poses");
    assert_eq!(poses.len(), 1);
    assert_eq!(poses[0].id, lighthouse_id);
    assert_eq!(poses[0].px, 1.0);

    let by_uuid = client
        .get_pose(domain_id, lighthouse_id)
        .await
        .expect("get pose by uuid");
    assert_eq!(by_uuid.short_id, short_id);

    let by_short = client
        .get_pose(domain_id, "shortid0001")
        .await
        .expect("get pose by short id");
    assert_eq!(by_short.id, lighthouse_id);
}

#[tokio::test]
async fn robot_token_refresh_reuses_cached_domain_token() {
    let dds = MockServer::start().await;
    let ds = MockServer::start().await;

    let robot_id = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let domain_id = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
    let exp = now_unix_secs() + 3600;

    let robot_jwt = make_jwt(json!({
        "exp": exp,
        "sub": robot_id,
        "node_id": robot_id,
        "assigned_domain_id": domain_id
    }));
    let domain_jwt = make_jwt(json!({
        "exp": exp,
        "sub": domain_id
    }));

    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/robot/verify"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "robot_id": robot_id,
            "access_token": robot_jwt,
            "access_expires_at": "2026-08-13T12:00:00Z"
        })))
        .expect(1)
        .mount(&dds)
        .await;

    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/robot/domain-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "domain_id": domain_id,
            "domain_server_url": ds.uri(),
            "access_token": domain_jwt,
            "access_expires_at": "2026-08-13T12:00:00Z"
        })))
        .expect(1)
        .mount(&dds)
        .await;

    Mock::given(method("GET"))
        .and(path(format!("/api/v1/domains/{}/lighthouses", domain_id)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "poses": []
        })))
        .expect(2)
        .mount(&ds)
        .await;

    let client = DomainClient::new_with_robot_credential(&dds.uri(), "robot-client", "secret")
        .await
        .unwrap();

    let _ = client.list_poses(domain_id).await.unwrap();
    let _ = client.list_poses(domain_id).await.unwrap();
}

#[tokio::test]
async fn robot_domain_token_forbidden_when_domain_mismatch_or_unbound() {
    let dds = MockServer::start().await;
    let robot_id = "cccccccc-cccc-cccc-cccc-cccccccccccc";
    let bound_domain = "dddddddd-dddd-dddd-dddd-dddddddddddd";
    let other_domain = "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee";
    let exp = now_unix_secs() + 3600;

    let bound_jwt = make_jwt(json!({
        "exp": exp,
        "sub": robot_id,
        "node_id": robot_id,
        "assigned_domain_id": bound_domain
    }));
    let unbound_jwt = make_jwt(json!({
        "exp": exp,
        "sub": robot_id,
        "node_id": robot_id
    }));

    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/robot/verify"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "robot_id": robot_id,
            "access_token": bound_jwt,
            "access_expires_at": "2026-08-13T12:00:00Z"
        })))
        .mount(&dds)
        .await;

    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/robot/domain-token"))
        .respond_with(
            ResponseTemplate::new(403).set_body_string("robot is not assigned to the requested domain"),
        )
        .mount(&dds)
        .await;

    let bound = DomainClient::new_with_robot_credential(&dds.uri(), "robot-client", "secret")
        .await
        .unwrap();
    let mismatch = bound.list_poses(other_domain).await.expect_err("mismatch");
    assert!(mismatch.to_string().contains("403"));

    let dds_unbound = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/robot/verify"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "robot_id": robot_id,
            "access_token": unbound_jwt,
            "access_expires_at": "2026-08-13T12:00:00Z"
        })))
        .mount(&dds_unbound)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/robot/domain-token"))
        .respond_with(
            ResponseTemplate::new(403).set_body_string("robot is not assigned to the requested domain"),
        )
        .mount(&dds_unbound)
        .await;

    let unbound = DomainClient::new_with_robot_credential(&dds_unbound.uri(), "robot-client", "secret")
        .await
        .unwrap();
    let err = unbound.list_poses(bound_domain).await.expect_err("unbound");
    assert!(err.to_string().contains("403"));
}

#[tokio::test]
async fn robot_verify_failure_surfaces_auki_error() {
    let dds = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/robot/verify"))
        .respond_with(ResponseTemplate::new(401).set_body_string("invalid robot credentials"))
        .mount(&dds)
        .await;

    let err = DomainClient::new_with_robot_credential(&dds.uri(), "robot-client", "bad-secret")
        .await
        .expect_err("verify should fail");
    assert!(err.to_string().contains("Failed to verify robot credentials"));
}

use jsonwebtoken::{encode, Algorithm, Header};
use posemesh_domain::auth::{encode_job_jwt, verify_token, DomainKeys, TaskTokenClaim};

#[cfg_attr(target_family = "wasm", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_family = "wasm"), test)]
fn generated_ed25519_keys_sign_and_verify_job_tokens() {
    let keys = DomainKeys::new(None).unwrap();
    let token = encode_job_jwt(
        &keys.private_key,
        "domain",
        "job",
        "task",
        "sender",
        "receiver",
    )
    .unwrap();
    let claims = verify_token::<TaskTokenClaim>(&token, &keys.public_key).unwrap();
    assert_eq!(claims.domain_id, "domain");
    assert_eq!(claims.job_id, "job");
    assert_eq!(claims.task_name, "task");
    assert_eq!(claims.sender, "sender");
    assert_eq!(claims.receiver, "receiver");

    let other_keys = DomainKeys::new(None).unwrap();
    assert!(verify_token::<TaskTokenClaim>(&token, &other_keys.public_key).is_err());
}

#[cfg_attr(target_family = "wasm", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_family = "wasm"), test)]
fn signed_expired_job_tokens_are_rejected() {
    let keys = DomainKeys::new(None).unwrap();
    let claims = serde_json::json!({
        "domain_id": "domain", "job_id": "job", "task_name": "task",
        "sender": "sender", "receiver": "receiver", "sub": "", "scope": "",
        "exp": 1, "iat": 0
    });
    let token = encode(&Header::new(Algorithm::EdDSA), &claims, &keys.private_key).unwrap();
    let error = verify_token::<TaskTokenClaim>(&token, &keys.public_key).unwrap_err();
    assert!(error.to_string().contains("ExpiredSignature"));
}

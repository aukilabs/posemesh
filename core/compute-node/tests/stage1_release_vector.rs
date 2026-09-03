//! Independent Posemesh check of the cross-repository Stage 1 release vector.
//!
//! The SDK owns the retained resource codec and carries the byte-identical
//! fixture. This consumer-side test verifies the identity, signed DDS claims,
//! mutual-authentication framing, and resource `0.2.0` payload without
//! importing SDK-internal test helpers.

use auki_p2p::{
    Identity, P2PAccessClaims, P2P_TOKEN_AUDIENCE, P2P_TOKEN_ISSUER, P2P_TOKEN_TTL, P2P_TOKEN_TYPE,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

const RESOURCES_V0_2_0: &str = "/auki/auth/1/resources/0.2.0";
const VECTOR_JSON: &str = include_str!("fixtures/stage1_cross_repository_vector.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stage1Vector {
    schema_version: u32,
    protocol_id: String,
    domain_id: String,
    identity_seed_hex: String,
    peer_id: String,
    dds_es256_public_key_pem: String,
    signed_claims: P2PAccessClaims,
    signed_credential: String,
    mutual_auth_frame_hex: String,
    resources_request_frame_hex: String,
    resources_response: Value,
    resources_response_frame_hex: String,
}

fn vector() -> Stage1Vector {
    serde_json::from_str(VECTOR_JSON).expect("Stage 1 vector must use the locked schema")
}

fn decode_hex(encoded: &str) -> Vec<u8> {
    hex::decode(encoded).expect("vector field must be hexadecimal")
}

fn identity_seed(encoded: &str) -> [u8; 32] {
    decode_hex(encoded)
        .try_into()
        .expect("identity seed must contain exactly 32 bytes")
}

fn authentication_transcript(credential: &str) -> Vec<u8> {
    let mut transcript = (credential.len() as u32).to_be_bytes().to_vec();
    transcript.extend_from_slice(credential.as_bytes());
    // Both sides emit this byte only after independently verifying the remote
    // credential and its binding to the Noise Peer ID.
    transcript.push(1);
    transcript
}

fn framed_payload(encoded: &str) -> Vec<u8> {
    let frame = decode_hex(encoded);
    let prefix: [u8; 4] = frame
        .get(..4)
        .expect("resource frame must contain a length prefix")
        .try_into()
        .unwrap();
    let declared = u32::from_be_bytes(prefix) as usize;
    let payload = frame[4..].to_vec();
    assert_eq!(declared, payload.len(), "resource frame length drifted");
    payload
}

#[test]
fn posemesh_matches_the_cross_repository_stage1_vector() {
    let fixture = vector();
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.protocol_id, RESOURCES_V0_2_0);

    let domain_id = Uuid::parse_str(&fixture.domain_id).expect("Domain ID must be a UUID");
    assert_eq!(domain_id.to_string(), fixture.domain_id);
    let identity = Identity::from_ed25519_seed(&identity_seed(&fixture.identity_seed_hex));
    assert_eq!(identity.peer_id().to_string(), fixture.peer_id);

    assert_eq!(fixture.signed_claims.peer_id, fixture.peer_id);
    assert_eq!(
        fixture.signed_claims.domain_ids,
        vec![fixture.domain_id.clone()]
    );
    assert_eq!(fixture.signed_claims.token_type, P2P_TOKEN_TYPE);
    assert_eq!(fixture.signed_claims.iss, P2P_TOKEN_ISSUER);
    assert_eq!(fixture.signed_claims.aud, [P2P_TOKEN_AUDIENCE]);
    assert_eq!(
        fixture.signed_claims.exp - fixture.signed_claims.iat,
        P2P_TOKEN_TTL.as_secs()
    );

    // This is a timeless signature vector, not live authority. Its historical
    // times remain signed and preserve the exact 30-minute lifetime contract.
    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = false;
    validation.validate_nbf = false;
    validation.set_audience(&[P2P_TOKEN_AUDIENCE]);
    validation.set_issuer(&[P2P_TOKEN_ISSUER]);
    let decoded = decode::<P2PAccessClaims>(
        &fixture.signed_credential,
        &DecodingKey::from_ec_pem(fixture.dds_es256_public_key_pem.as_bytes()).unwrap(),
        &validation,
    )
    .expect("DDS signature and claim profile must remain fixed");
    assert_eq!(decoded.claims, fixture.signed_claims);
    assert_eq!(
        authentication_transcript(&fixture.signed_credential),
        decode_hex(&fixture.mutual_auth_frame_hex)
    );

    let request: Value =
        serde_json::from_slice(&framed_payload(&fixture.resources_request_frame_hex))
            .expect("resource request must be JSON");
    assert_eq!(request, json!({}));

    let response: Value =
        serde_json::from_slice(&framed_payload(&fixture.resources_response_frame_hex))
            .expect("resource response must be JSON");
    assert_eq!(response, fixture.resources_response);
    let resources = response["resources"]
        .as_array()
        .expect("resource response must contain an array");
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0]["writer_peer_id"], fixture.peer_id);
    assert_eq!(resources[0]["resource_id"], "stage1-camera");
}

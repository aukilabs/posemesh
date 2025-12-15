use chrono::{DateTime, Utc};
use hex::FromHexError;
use k256::ecdsa::{self, SigningKey};
use k256::FieldBytes;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use thiserror::Error;

const API_PREFIX: &str = "/internal/v1";
const REQUEST_PATH: &str = "/auth/siwe/request";
const VERIFY_PATH: &str = "/auth/siwe/verify";

#[derive(Debug, Error)]
pub enum SiweError {
    #[error("invalid private key hex: {0}")]
    InvalidHex(FromHexError),
    #[error("invalid private key length: expected 32 bytes, got {0}")]
    InvalidPrivateKeyLength(usize),
    #[error("failed to initialize signing key: {0}")]
    InvalidSigningKey(ecdsa::Error),
    #[error("failed to sign SIWE message: {0}")]
    Signing(ecdsa::Error),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    InvalidExpiration(#[from] chrono::ParseError),
    #[error("missing field '{0}' in response")]
    MissingField(&'static str),
}

pub type Result<T> = std::result::Result<T, SiweError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessBundle {
    token: String,
    expires_at: DateTime<Utc>,
}

impl AccessBundle {
    pub fn new(token: impl Into<String>, expires_at: DateTime<Utc>) -> Self {
        Self {
            token: token.into(),
            expires_at,
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}

pub fn sign_message(priv_hex: &str, message: &str) -> Result<String> {
    let key_bytes = decode_priv_key(priv_hex)?;
    let signing_key = signing_key_from_bytes(&key_bytes)?;
    let digest = ethereum_message_digest(message);
    let (signature, recovery_id) = signing_key
        .sign_digest_recoverable(digest)
        .map_err(SiweError::Signing)?;
    let mut bytes = [0u8; 65];
    let signature_bytes: [u8; 64] = signature.to_bytes().into();
    bytes[..64].copy_from_slice(&signature_bytes);
    // Ethereum expects recovery id in {27, 28} form.
    let recovery_id = recovery_id.to_byte() + 27;
    bytes[64] = recovery_id;
    Ok(format!("0x{}", hex::encode(bytes)))
}

/// Fields returned by DDS SIWE nonce endpoint. These should be used to build
/// a standards-compliant SIWE message string for signing.
#[derive(Deserialize, Debug, Clone)]
pub struct SiweRequestMeta {
    pub nonce: Option<String>,
    pub domain: Option<String>,
    pub uri: Option<String>,
    pub version: Option<String>,
    #[serde(rename = "chainId")]
    pub chain_id: Option<i64>,
    #[serde(rename = "issuedAt")]
    pub issued_at: Option<String>,
}

/// Request a SIWE nonce from DDS, binding it to the provided wallet address.
pub async fn request_nonce(base_url: &str, wallet: &str) -> Result<SiweRequestMeta> {
    let client = new_client()?;
    let url = endpoint(base_url, REQUEST_PATH);
    let response = client
        .post(url)
        .json(&serde_json::json!({ "wallet": wallet }))
        .send()
        .await?
        .error_for_status()?;
    let body: SiweRequestMeta = response.json().await?;
    if body.nonce.as_deref().unwrap_or("").is_empty() {
        return Err(SiweError::MissingField("nonce"));
    }
    Ok(body)
}

#[derive(Serialize)]
struct VerifyRequest<'a> {
    address: &'a str,
    message: &'a str,
    signature: &'a str,
}

#[derive(Deserialize)]
struct VerifyResponse {
    access_token: Option<String>,
    expires_at: Option<String>,
    access_expires_at: Option<String>,
}

pub async fn verify(
    base_url: &str,
    address: &str,
    message: &str,
    signature: &str,
) -> Result<AccessBundle> {
    let client = new_client()?;
    let url = endpoint(base_url, VERIFY_PATH);
    let payload = VerifyRequest {
        address,
        message,
        signature,
    };
    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await?
        .error_for_status()?;
    let body: VerifyResponse = response.json().await?;
    let token = body
        .access_token
        .filter(|value| !value.is_empty())
        .ok_or(SiweError::MissingField("access_token"))?;
    let expires_at_raw = body
        .access_expires_at
        .or(body.expires_at)
        .ok_or(SiweError::MissingField("access_expires_at"))?;
    let expires_at = DateTime::parse_from_rfc3339(&expires_at_raw)?.with_timezone(&Utc);
    Ok(AccessBundle { token, expires_at })
}

fn decode_priv_key(priv_hex: &str) -> Result<[u8; 32]> {
    let trimmed = priv_hex.strip_prefix("0x").unwrap_or(priv_hex);
    let bytes = hex::decode(trimmed).map_err(SiweError::InvalidHex)?;
    if bytes.len() != 32 {
        return Err(SiweError::InvalidPrivateKeyLength(bytes.len()));
    }
    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes);
    Ok(result)
}

fn endpoint(base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{base}{API_PREFIX}{path}")
}

fn ethereum_message_digest(message: &str) -> Keccak256 {
    let mut digest = Keccak256::new();
    let prefix = format!("\u{19}Ethereum Signed Message:\n{}", message.len());
    digest.update(prefix.as_bytes());
    digest.update(message.as_bytes());
    digest
}

fn new_client() -> Result<Client> {
    Client::builder()
        .no_proxy()
        .build()
        .map_err(SiweError::Request)
}

fn signing_key_from_bytes(bytes: &[u8; 32]) -> Result<SigningKey> {
    let field_bytes: FieldBytes = (*bytes).into();
    SigningKey::from_bytes(&field_bytes).map_err(SiweError::InvalidSigningKey)
}

/// Compose a SIWE message string using values returned from DDS and the signer
/// address. Optionally include a list of resource URNs.
pub fn compose_message(
    meta: &SiweRequestMeta,
    address: &str,
    resources: Option<&[&str]>,
) -> Result<String> {
    let domain = meta
        .domain
        .as_deref()
        .ok_or(SiweError::MissingField("domain"))?;
    let uri = meta.uri.as_deref().ok_or(SiweError::MissingField("uri"))?;
    let version = meta
        .version
        .as_deref()
        .ok_or(SiweError::MissingField("version"))?;
    let chain_id = meta.chain_id.ok_or(SiweError::MissingField("chainId"))?;
    let nonce = meta
        .nonce
        .as_deref()
        .ok_or(SiweError::MissingField("nonce"))?;
    let issued_at = meta
        .issued_at
        .as_deref()
        .ok_or(SiweError::MissingField("issuedAt"))?;

    let mut out = String::new();
    out.push_str(&format!(
        "{} wants you to sign in with your Ethereum account:\n",
        domain
    ));
    out.push_str(address);
    out.push_str("\n\n");
    out.push_str(&format!("URI: {}\n", uri));
    out.push_str(&format!("Version: {}\n", version));
    out.push_str(&format!("Chain ID: {}\n", chain_id));
    out.push_str(&format!("Nonce: {}\n", nonce));
    out.push_str(&format!("Issued At: {}", issued_at));
    if let Some(list) = resources {
        if !list.is_empty() {
            out.push_str("\nResources:\n");
            for r in list {
                out.push_str(&format!("- {}\n", r));
            }
            if out.ends_with('\n') {
                out.pop();
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PRIV_HEX: &str = "4c0883a69102937d6231471b5dbb6204fe5129617082798ce3f4fdf2548b6f90";
    const TEST_ADDRESS: &str = "0xfdbb6caf01414300c16ea14859fec7736d95355f";

    #[test]
    fn signs_message_with_expected_signature() {
        let message = format!(
            "example.com wants you to sign in with your Ethereum account:\n{}\n\nURI: https://example.com\nVersion: 1\nChain ID: 1\nNonce: abc123\nIssued At: 2024-05-01T00:00:00Z",
            TEST_ADDRESS
        );

        let signature = sign_message(TEST_PRIV_HEX, &message).expect("signature");

        let expected_signature = "0x390786f1c4840ec337aef7c4a6d15bba128cc308e2f32f4528e2f8dd44f61f354a68a45bcf7d39eb724346a02f6eefebd8a39e20ebf68435bfe026ed47b1e5171b";
        assert_eq!(signature, expected_signature);
    }

    #[test]
    fn compose_message_includes_resources() {
        let meta = SiweRequestMeta {
            nonce: Some("nonce".into()),
            domain: Some("example.com".into()),
            uri: Some("https://example.com".into()),
            version: Some("1".into()),
            chain_id: Some(1),
            issued_at: Some("2024-05-01T00:00:00Z".into()),
        };
        let msg =
            compose_message(&meta, TEST_ADDRESS, Some(&["urn:foo", "urn:bar"])).expect("message");
        assert!(msg.contains("Resources:\n- urn:foo\n- urn:bar"));
    }
}

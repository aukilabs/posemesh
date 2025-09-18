use chrono::{DateTime, SecondsFormat, Utc};
use secp256k1::{ecdsa::Signature, Message, PublicKey, Secp256k1, SecretKey};
use sha2::{Digest as Sha2Digest, Sha256};
use sha3::Keccak256;

/// Load a secp256k1 private key from lowercase hex (optionally 0x-prefixed).
pub fn load_secp256k1_privhex(hex_str: &str) -> anyhow::Result<SecretKey> {
    let s = hex_str.trim();
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s)?;
    if bytes.len() != 32 {
        anyhow::bail!("invalid secp256k1 secret length: {}", bytes.len());
    }
    let sk = SecretKey::from_slice(&bytes)?;
    Ok(sk)
}

/// Derive the uncompressed public key (0x04 || X || Y) as lowercase hex.
pub fn secp256k1_pubkey_uncompressed_hex(sk: &SecretKey) -> String {
    let secp = Secp256k1::new();
    let pk = PublicKey::from_secret_key(&secp, sk);
    let uncompressed = pk.serialize_uncompressed(); // 65 bytes, leading 0x04
    hex::encode(uncompressed)
}

/// Sign arbitrary message bytes using RFC6979 deterministic ECDSA over SHA-256.
/// Returns the compact 64-byte signature as lowercase hex (r||s).
pub fn sign_compact_hex(sk: &SecretKey, msg: &[u8]) -> String {
    let digest = Sha256::digest(msg);
    let message = Message::from_digest_slice(&digest).expect("sha256 is 32 bytes");
    let secp = Secp256k1::new();
    let sig: Signature = secp.sign_ecdsa(&message, sk);
    let compact = sig.serialize_compact();
    hex::encode(compact)
}

/// Sign using Ethereum-style Keccak-256 digest and return 65-byte (r||s||v) hex.
pub fn sign_recoverable_keccak_hex(sk: &SecretKey, msg: &[u8]) -> String {
    // Keccak-256 of the raw message bytes (no prefixing)
    let mut hasher = Keccak256::new();
    hasher.update(msg);
    let hash = hasher.finalize();
    let message = Message::from_digest_slice(&hash).expect("keccak256 is 32 bytes");
    let secp = Secp256k1::new();
    let rsig = secp.sign_ecdsa_recoverable(&message, sk);
    let (rid, sig_bytes) = rsig.serialize_compact();
    let mut out = [0u8; 65];
    out[..64].copy_from_slice(&sig_bytes);
    out[64] = rid.to_i32() as u8; // 0 or 1
    hex::encode(out)
}

/// RFC3339 with nanoseconds and Z suffix.
pub fn format_timestamp_nanos(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    #[test]
    fn timestamp_nanos_format() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
        let time = NaiveTime::from_hms_nano_opt(3, 4, 5, 6_007_008).unwrap();
        let dt = DateTime::<Utc>::from_naive_utc_and_offset(date.and_time(time), Utc);
        let s = format_timestamp_nanos(dt);
        assert_eq!(s, "2024-01-02T03:04:05.006007008Z");
    }

    #[test]
    fn sign_fixed_keccak_recoverable_hex_has_expected_shape() {
        // Fixed key and message; ensure output shape and stability invariants.
        let sk = load_secp256k1_privhex(
            "e331b6d69882b4ed5bb7f55b585d7d0f7dc3aeca4a3deee8d16bde3eca51aace",
        )
        .expect("key");
        let url = "https://node.example.com";
        let ts = "2024-01-02T03:04:05.000000000Z";
        let msg = format!("{}{}", url, ts);
        let sig = sign_recoverable_keccak_hex(&sk, msg.as_bytes());
        // Expect 65-byte signature hex => 130 hex chars
        assert_eq!(sig.len(), 130);
        // All lowercase hex
        assert!(sig
            .chars()
            .all(|c| c.is_ascii_hexdigit() && c.is_ascii_lowercase() || c.is_ascii_digit()));
    }
}

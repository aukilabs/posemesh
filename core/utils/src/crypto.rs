use std::{io::Error, io::ErrorKind};
use k256::ecdsa::{Signature, SigningKey};

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Hex decoding error: {0}")]
    HexError(#[from] hex::FromHexError),

    #[error("Signing error: {0}")]
    SigningError(#[from] k256::ecdsa::signature::Error),

    #[error("No private key provided")]
    NoPrivateKey,
}


#[derive(Debug)]
pub struct Secp256k1KeyPair {
    signing_key: SigningKey,
}

impl Secp256k1KeyPair {
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.signing_key.verifying_key().to_encoded_point(false).as_bytes())
    }
}

#[cfg(not(target_family="wasm"))]
fn keypair_file(private_key_path: &str) -> Result<Secp256k1KeyPair, CryptoError> {
    use std::{fs, io::Read, path::Path};

    let path = Path::new(private_key_path);
    // Check if the keypair file exists
    let mut file = fs::File::open(path)?;
    // Read the keypair from the file
    let mut file_private_key = "".to_string();
    file.read_to_string(&mut file_private_key)?;
    let private_key = file_private_key.strip_prefix("0x").unwrap_or(&file_private_key).trim();
    let private_key = hex::decode(private_key)?;
    let signing_key = SigningKey::from_slice(&private_key)?;
    return Ok(Secp256k1KeyPair { signing_key });
}

pub fn parse_secp256k1_private_key(
    private_key: Option<&str>,
    private_key_path: Option<&str>,
) -> Result<Secp256k1KeyPair, CryptoError> {
    match private_key {
        Some(private_key) => {
            let removed_0x_prefix = private_key.strip_prefix("0x").unwrap_or(private_key);
            let private_key_bytes = hex::decode(removed_0x_prefix)?;
            let signing_key = SigningKey::from_slice(&private_key_bytes)?;

            #[cfg(not(target_family="wasm"))]
            if let Some(path) = private_key_path {
                use std::{fs, path::Path, io::Write};

                let path = Path::new(path);
                if let Some(parent) = path.parent() {

                    if let Err(err) = fs::create_dir_all(parent) {
                        tracing::error!("Failed to create directory: {err}");
                    }

                    let mut file = fs::File::create(path)?;
                    file.write_all(private_key.as_bytes())?;
                }
            }
            return Ok(Secp256k1KeyPair { signing_key });
        }
        None => {
            #[cfg(not(target_family="wasm"))]
            if let Some(key_path) = private_key_path {
                return keypair_file(key_path);
            }
            return Err(CryptoError::NoPrivateKey);
        }
    }
}


pub fn sign_message(message: &str, keypair: &Secp256k1KeyPair) -> Result<String, Error> {
    use tiny_keccak::Hasher;
    // Hash the message using Keccak-256
    let mut hasher = tiny_keccak::Keccak::v256();
    hasher.update(message.as_bytes());
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    // Sign the hash and return signature bytes
    let signature: Signature = keypair.signing_key.sign_prehash_recoverable(&hash).map_err(|e| Error::new(ErrorKind::Other, e))?.0;
    let signature = signature.to_bytes().to_vec();
    let hex_signature = hex::encode(signature);
    Ok(format!("0x{}", hex_signature))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_or_create_keypair_with_private_key() {
        let private_key = "2719d2c0d35fa8a6dce9622e480764ecc0428dd10c70cc52ec0349351989d27a";
        let keypair = parse_secp256k1_private_key(Some(private_key), None).unwrap();
        
        // Verify the keypair was created with the correct private key
        let expected_bytes = hex::decode(private_key).unwrap();
        assert_eq!(keypair.signing_key.to_bytes().to_vec(), expected_bytes);
    }

    #[test]
    fn test_parse_or_create_keypair_with_0x_private_key() {
        let private_key = "0x2719d2c0d35fa8a6dce9622e480764ecc0428dd10c70cc52ec0349351989d27a";
        let keypair = parse_secp256k1_private_key(Some(private_key), None).unwrap();
        
        // Verify the keypair was created with the correct private key
        let expected_bytes = hex::decode(private_key.strip_prefix("0x").unwrap()).unwrap();
        assert_eq!(keypair.signing_key.to_bytes().to_vec(), expected_bytes);
    }

    #[test]
    fn test_parse_ecdsa_private_key_with_no_inputs() {
        parse_secp256k1_private_key(None, None).expect_err("it should return error");
    }

    #[test]
    fn test_parse_ecdsa_private_key_with_file() {
        let temp_dir = std::env::temp_dir();
        let key_path = temp_dir.join("test_key.pkey");
        
        // Write private key to file
        std::fs::write(&key_path, "2719d2c0d35fa8a6dce9622e480764ecc0428dd10c70cc52ec0349351989d27a").unwrap();

        let keypair = parse_secp256k1_private_key(None, Some(key_path.to_str().unwrap())).unwrap();
        
        // Verify the keypair was created with the correct private key
        let expected_bytes = hex::decode("2719d2c0d35fa8a6dce9622e480764ecc0428dd10c70cc52ec0349351989d27a").unwrap();
        assert_eq!(keypair.signing_key.to_bytes().to_vec(), expected_bytes);

        // Cleanup
        std::fs::remove_file(key_path).unwrap();
    }

    #[test]
    fn test_sign_message() {
        let keypair = parse_secp256k1_private_key(Some("2719d2c0d35fa8a6dce9622e480764ecc0428dd10c70cc52ec0349351989d27a"), None).unwrap();
        let signature = sign_message("test message", &keypair).unwrap();
        assert_eq!(signature, "0x2dcb35d237f7a1d954aceffbaf7cf6e2d36f947c4a83785117c322b7bab031a8180a24829c9a80fc690de79624088a0fa7f62af48407151818299076ecb08af7");
    }
}

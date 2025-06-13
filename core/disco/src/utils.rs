use crate::{error::DiscoveryError, protobuf::{msgpb::{msg::Payload, Msg}, nodepb::RegisterNodeRequest}};
use k256::ecdsa::{Signature, SigningKey};
use libp2p_identity::secp256k1::Keypair;
use prost::Message;
use uuid::Uuid;

pub fn handle_message(bytes: &[u8]) -> Result<(), DiscoveryError> {
    let msg = Msg::decode(bytes)?;
    match msg.payload {
        Some(Payload::RegisterNodeResponse(res)) => {
            tracing::info!("received RegisterNodeResponse: {:?}", res);
        },
        _ => {}
    }
    Ok(())
}

pub fn new_register_node_request_v1(centralized_id: &str, keypair: &Keypair, addrs: Vec<String>) -> Msg {
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S.%fZ").to_string();
    let public_key = hex::encode(keypair.public().to_bytes_uncompressed());

    let message = format!("{}{}", centralized_id, timestamp);
    let signature = sign_message(&message, keypair).unwrap();

    return Msg {
        req_id: Uuid::new_v4().to_string(),
        payload: Some(Payload::RegisterNodeRequest(RegisterNodeRequest {
            version: "v0.0.0".to_string(),
            signature,
            timestamp,
            public_key,
            capabilities: "{\"capabilities\": []}".to_string(),
            addrs
        }))
    };
}

pub fn sign_message(message: &str, keypair: &Keypair) -> Result<String, DiscoveryError> {
    use tiny_keccak::Hasher;
    // Hash the message using Keccak-256
    let mut hasher = tiny_keccak::Keccak::v256();
    hasher.update(message.as_bytes());
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    // Convert private key bytes to SigningKey
    let secret_key = keypair.secret().to_bytes();
    let signing_key = SigningKey::from_slice(&secret_key)?;

    // Sign the hash and return signature bytes
    let signature: Signature = signing_key.sign_prehash_recoverable(&hash)?.0;
    let signature = signature.to_bytes().to_vec();
    let hex_signature = hex::encode(signature);
    Ok(format!("0x{}", hex_signature))
}


#[cfg(test)]
mod tests {
    use super::*;
    use posemesh_utils::parse_secp256k1_private_key;

    #[test]
    fn test_sign_message() {
        let keypair = parse_secp256k1_private_key(Some("2719d2c0d35fa8a6dce9622e480764ecc0428dd10c70cc52ec0349351989d27a"), None).unwrap();
        let signature = sign_message("test message", &keypair).unwrap();
        assert_eq!(signature, "0x2dcb35d237f7a1d954aceffbaf7cf6e2d36f947c4a83785117c322b7bab031a8180a24829c9a80fc690de79624088a0fa7f62af48407151818299076ecb08af7");
    }
}

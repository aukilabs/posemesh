use crate::{error::DiscoveryError, protobuf::{msgpb::{msg::Payload, Msg}, nodepb::RegisterNodeRequest}};
use posemesh_utils::crypto::{sign_message, Secp256k1KeyPair};
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

pub fn new_register_node_request_v1(centralized_id: &str, keypair: &Secp256k1KeyPair, addrs: Vec<String>) -> Msg {
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S.%fZ").to_string();
    let public_key = keypair.public_key_hex();

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


use crate::{message::read_prefix_size_message, protobuf::task::DomainClusterHandshake};
use networking::AsyncStream;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, EncodingKey, Header, decode, DecodingKey, Validation, Algorithm};
use std::time::{SystemTime, Duration, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskTokenClaim {
    pub domain_id: String,
    pub task_name: String,
    pub job_id: String,
    pub sender: String,
    pub receiver: String,
    pub exp: usize,
    pub iat: usize,
    pub sub: String,
}

pub fn decode_jwt(token: &str) -> Result<TaskTokenClaim, Box<dyn std::error::Error + Send + Sync>> {
    let token_data = decode::<TaskTokenClaim>(token, &DecodingKey::from_secret("secret".as_ref()), &Validation::new(Algorithm::HS256))?;
    Ok(token_data.claims)
}

pub async fn handshake<S: AsyncStream>(stream: &mut S) -> Result<TaskTokenClaim, Box<dyn std::error::Error + Send + Sync>> {
    let header = read_prefix_size_message::<DomainClusterHandshake>(stream).await?;
    decode_jwt(header.access_token.as_str())
}

pub fn encode_jwt(domain_id: &str, job_id: &str, task_name: &str, sender: &str, receiver: &str, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let exp = now + Duration::from_secs(60*60);
    let claims = TaskTokenClaim {
        domain_id: domain_id.to_string(),
        task_name: task_name.to_string(),
        sender: sender.to_string(),
        receiver: receiver.to_string(),
        job_id: job_id.to_string(),
        // TODO: set exp, iat, sub and scope
        exp: exp.as_secs() as usize,
        iat: 0,
        sub: "".to_string(),
    };

    // TODO: use ed25519 or ethereum key instead
    let token = encode(
        &Header::default(),
        &claims,
           &EncodingKey::from_secret(secret.as_ref()),
    )?;

    Ok(token)
}

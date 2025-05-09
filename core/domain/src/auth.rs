use crate::{capabilities::public_key::{PublicKeyStorage, PUBLIC_KEY_PROTOCOL_V1}, message::{read_prefix_size_message, request_response_raw}, protobuf::task::DomainClusterHandshake};
use async_timer::Interval;
use base64::Engine;
use networking::{client::Client, libp2p::NetworkError, AsyncStream};
use ring::{error, signature::{Ed25519KeyPair, KeyPair}};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use jsonwebtoken::{encode, EncodingKey, Header, decode, DecodingKey, Validation, Algorithm};
use std::{sync::Arc, time::{Duration, SystemTime, UNIX_EPOCH}};
use futures::{channel::oneshot, lock::Mutex, select, FutureExt};

#[cfg(not(target_family = "wasm"))]
use tokio::spawn;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

#[derive(Clone)]
pub struct DomainKeys {
    pub private_key: EncodingKey,
    pub public_key: Vec<u8>,
}


fn decode_pem(private_key_pem: &str) -> Result<DomainKeys, AuthError> {
    let key = EncodingKey::from_ed_pem(private_key_pem.as_bytes())?;
    
    // Remove the PEM headers and footers
    let pem_stripped = private_key_pem 
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<Vec<&str>>()
        .join("");

    // Decode the base64 content
    let der_bytes = base64::engine::general_purpose::STANDARD.decode(pem_stripped)?; 
    let keypair = Ed25519KeyPair::from_pkcs8(&der_bytes)?;
    let public_key = keypair.public_key().as_ref().to_vec();
    Ok(DomainKeys { private_key: key, public_key })
}

impl DomainKeys {
    pub fn new(private_key_path: Option<String>) -> Result<Self, AuthError> {
        let rng = ring::rand::SystemRandom::new();
        let keypair = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();

        let der_bytes = keypair.as_ref();
        let base64_encoded = base64::engine::general_purpose::STANDARD.encode(der_bytes);
        let pem = format!(
            "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----",
            base64_encoded
                .as_bytes()
                .chunks(64)
                .map(std::str::from_utf8)
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .join("\n")
        ); 

        let private_key = EncodingKey::from_ed_pem(pem.as_bytes())?;
        let public_key = Ed25519KeyPair::from_pkcs8(der_bytes).unwrap().public_key().as_ref().to_vec();

        if let Some(private_key_path) = private_key_path {
            std::fs::write(private_key_path, pem)?;
        }
        Ok(Self { private_key, public_key })
    }

    pub fn from_file(private_key_path: &str) -> Result<Self, AuthError> {
        if let Ok(private_key) = std::fs::read_to_string(private_key_path) {
            decode_pem(&private_key)
        } else {
            Self::new(Some(private_key_path.to_string()))
        }
    }

    pub fn from_pem(private_key_pem: &str) -> Result<Self, AuthError> {
        decode_pem(private_key_pem)
    }
    
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("JWT error: {0}")]
    JWTError(#[from] jsonwebtoken::errors::Error),
    #[error("Key pair error: {0}")]
    KeyPairError(#[from] error::KeyRejected),
    #[error("Base64 error: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("Private key is required")]
    PrivateKeyRequired,
    #[error("Domain ID mismatch in token")]
    DomainIdMismatch,
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Network error: {0}")]
    NetworkError(#[from] NetworkError),
    #[error("Public key for domain {0} not found")]
    PublicKeyNotFound(String),
    #[error("Public key for domain {0} already exists")]
    PublicKeyAlreadyExists(String),
    #[error("Protobuf error: {0}")]
    ProtobufError(#[from] quick_protobuf::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskTokenClaim {
    pub domain_id: String,
    pub task_name: String,
    pub job_id: String,
    pub sender: String,
    pub receiver: String,
    exp: usize,
    iat: usize,
    pub sub: String,
    pub scope: String,
}

impl TokenClaim for TaskTokenClaim {
    fn add_ttl(&mut self, ttl: Duration) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
        self.exp = (now + ttl).as_secs() as usize;
        self.iat = now.as_secs() as usize;
    }
}
pub trait TokenClaim: Serialize + DeserializeOwned {
    fn add_ttl(&mut self, ttl: Duration);
}

pub async fn handshake<S: AsyncStream, P: PublicKeyStorage>(stream: &mut S, key_loader: P) -> Result<TaskTokenClaim, AuthError> {
    let header = read_prefix_size_message::<DomainClusterHandshake>(stream).await?;
    let public_key = key_loader.get_by_domain_id(header.domain_id.clone()).await?;
    let claim = verify_token::<TaskTokenClaim>(&header.access_token, &public_key)?;
    if claim.domain_id != header.domain_id {
        return Err(AuthError::DomainIdMismatch);
    }
    Ok(claim)
}

pub fn verify_token<C: TokenClaim>(token: &str, public_key: &[u8]) -> Result<C, AuthError> {
    let mut validator = Validation::new(Algorithm::EdDSA);
    validator.leeway = 5;
    validator.validate_exp = true;
    let token_data = decode::<C>(token, &DecodingKey::from_ed_der(public_key), &validator)?;
    Ok(token_data.claims)
}

pub fn encode_jwt<C: TokenClaim>(claim: &mut C, private_key: &EncodingKey, ttl: Duration) -> Result<String, AuthError> {
    claim.add_ttl(ttl);
    encode(
        &Header::new(Algorithm::EdDSA),
        claim,
        private_key,
    ).map_err(|e| AuthError::JWTError(e))
}

pub fn encode_job_jwt(private_key: &EncodingKey, domain_id: &str, job_id: &str, task_name: &str, sender: &str, receiver: &str) -> Result<String, AuthError> {
    let mut claims = TaskTokenClaim {
        domain_id: domain_id.to_string(),   
        task_name: task_name.to_string(),
        sender: sender.to_string(),
        receiver: receiver.to_string(),
        job_id: job_id.to_string(),
        exp: 0,
        iat: 0,
        sub: "".to_string(),
        scope: "".to_string(),
    };

    encode_jwt(&mut claims, &private_key, Duration::from_secs(3600))
}

pub struct AuthServer {
    pub default_token_ttl: Duration,
    pub keys: DomainKeys,
}

impl AuthServer {
    pub fn new(private_key_pem: Option<String>, private_key_pem_path: Option<String>, default_token_ttl: Duration) -> Result<Self, AuthError> {
        let keys = if let Some(private_key_pem_path) = private_key_pem_path {
            DomainKeys::from_file(private_key_pem_path.as_str())?
        } else if let Some(private_key_pem) = private_key_pem {
            DomainKeys::from_pem(private_key_pem.as_str())?
        } else {
            return Err(AuthError::PrivateKeyRequired);
        };
        Ok(Self { default_token_ttl, keys })
    }

    pub fn generate_token<C: TokenClaim>(&self, claim: &mut C, ttl: Option<Duration>) -> Result<String, AuthError> {
        encode_jwt(claim, &self.private_key(), ttl.unwrap_or(self.default_token_ttl))
    }

    pub fn public_key(&self) -> &[u8] {
        &self.keys.public_key
    }

    pub fn private_key(&self) -> &EncodingKey {
        &self.keys.private_key
    }
}

pub struct AuthClient {
    public_key: Arc<Mutex<Vec<u8>>>,
    tx: Option<oneshot::Sender<()>>,
}

impl AuthClient {
    pub async fn initialize(c: Client, public_key_peer: &str, cache_ttl: Duration, domain_id: &str) -> Result<Self, AuthError> {
        let public_key = request_response_raw(c.clone(), public_key_peer, PUBLIC_KEY_PROTOCOL_V1, &domain_id.as_bytes(), Duration::from_secs(2).as_millis() as u32).await?;

        let c_clone = c.clone();
        let (tx, rx) = oneshot::channel::<()>();
        let public_key = Arc::new(Mutex::new(public_key));

        let mut interval = Interval::platform_new(cache_ttl);
        let public_key_clone = public_key.clone();
        let public_key_peer_clone = public_key_peer.to_string();
        let mut rx = rx.fuse();
        spawn(async move {
            loop {
                select! {
                    _ = interval.as_mut().fuse() => {
                        let public_key = request_response_raw(c_clone.clone(), &public_key_peer_clone, PUBLIC_KEY_PROTOCOL_V1, &vec![], Duration::from_secs(2).as_millis() as u32).await.expect("Failed to get public key");
                        *public_key_clone.lock().await = public_key;
                    }
                    _ = rx => {
                        interval.cancel();
                        break;
                    }
                }
            }
        });
        Ok(Self { public_key, tx: Some(tx) })
    }

    pub async fn initialize_with_known_public_key(public_key: Vec<u8>) -> Self {
        Self { public_key: Arc::new(Mutex::new(public_key)), tx: None }
    }

    pub async fn verify_token<C: TokenClaim>(&self, token: &str) -> Result<C, AuthError> {
        let public_key = self.public_key.lock().await;
        verify_token::<C>(token, public_key.as_ref())
    }

    pub async fn public_key(&self) -> Vec<u8> {
        let public_key = self.public_key.lock().await;
        public_key.clone()
    }
}

impl Drop for AuthClient {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(());
        }
    }
}

#[cfg(not(target_family = "wasm"))]
mod tests {
    use crate::capabilities::public_key::{self, serve_public_key_v1, PublicKeyStorage};

    use super::*;
    use async_trait::async_trait;
    use networking::libp2p::{Networking, NetworkingConfig};
    use std::fs::{create_dir_all, remove_dir_all};
    use tokio::{self, select, spawn, time::sleep};
    use futures::{StreamExt, AsyncWriteExt};

    struct TestContext {
        base_dir: String,
        client: Networking,
        server: Networking,
    }
    impl TestContext {
        pub async fn new() -> Self {
            let base_dir = "volume".to_string();
            create_dir_all("volume").unwrap();
            let mut server_cfg = NetworkingConfig::default();
            server_cfg.port = 10000;
            server_cfg.private_key_path = Some("volume/server.pkey".to_string());
            let server = Networking::new(&server_cfg).unwrap();
            let mut client_cfg = NetworkingConfig::default();
            client_cfg.port = 10001;
            client_cfg.private_key_path = Some("volume/client.pkey".to_string());
            client_cfg.bootstrap_nodes = vec![format!("/ip4/127.0.0.1/tcp/10000/p2p/{}", server.id.clone())];
            let client = Networking::new(&client_cfg).unwrap();
            sleep(Duration::from_secs(2)).await;
            Self {
                base_dir,
                client,
                server,
            }
        }
    }
    impl Drop for TestContext {
        fn drop(&mut self) {
            remove_dir_all(self.base_dir.clone()).unwrap();

            let mut client = self.client.clone();
            let mut server = self.server.clone();
            spawn(async move {
                let _ = client.client.cancel().await;
            });
            spawn(async move {
                let _ = server.client.cancel().await;
            });
        }
    }
    #[tokio::test]
    async fn test_auth_client() {
        let mut ctx = TestContext::new().await;
        let mut claim = TaskTokenClaim {
            domain_id: "domain_id".to_string(),
            task_name: "task_name".to_string(),
            job_id: "job_id".to_string(),
            sender: "sender".to_string(),
            receiver: "receiver".to_string(),
            exp: 0,
            iat: 0,
            sub: "sub".to_string(),
            scope: "scope".to_string()
        };

        let ttl = Duration::from_secs(3);
        let auth_server      = AuthServer::new(None, Some("volume/domain.pkey".to_string()), ttl).expect("failed to initialize auth server");
        let mut public_key_proto = ctx.server.client.set_stream_handler(PUBLIC_KEY_PROTOCOL_V1.to_string()).await.expect("failed to add /public-key");
        
        let public_key = auth_server.public_key().clone();
        #[derive(Clone)]
        struct TestPublicKeyStorage {
            public_key: Vec<u8>,
        }
        #[async_trait]
        impl PublicKeyStorage for TestPublicKeyStorage {
            async fn get_by_domain_id(&self, _: String) -> Result<Vec<u8>, AuthError> {
                Ok(self.public_key.clone())
            }
        }
        let pubkey_storage = TestPublicKeyStorage {public_key: public_key.to_vec()};
        spawn(async move {
            loop {
                select! {
                    Some((_, stream)) = public_key_proto.next() => {
                        let pubkey_storage = pubkey_storage.clone();
                        spawn(serve_public_key_v1(stream, pubkey_storage));
                    }
                    else => break
                }
            }
        });
        let token = auth_server.generate_token::<TaskTokenClaim>(&mut claim, None).expect("failed to generate token");
        
        let auth_client = AuthClient::initialize(ctx.client.client.clone(), ctx.server.id.as_str(), ttl, "domain_id").await.unwrap();
        let parsed_claim = auth_client.verify_token::<TaskTokenClaim>(&token).await.expect("failed to verify token");

        assert_ne!(auth_server.public_key().len(), 0);
        assert_eq!(auth_server.public_key(), auth_client.public_key().await);
        assert_eq!(parsed_claim.domain_id, claim.domain_id);
        assert_eq!(parsed_claim.task_name, claim.task_name);
        assert_eq!(parsed_claim.job_id, claim.job_id);
        assert_eq!(parsed_claim.sender, claim.sender);
        assert_eq!(parsed_claim.receiver, claim.receiver);
        assert_eq!(parsed_claim.sub, claim.sub);
        assert_eq!(parsed_claim.scope, claim.scope);
        assert!(parsed_claim.exp > 0);
        assert!(parsed_claim.iat > 0);

        sleep(ttl + Duration::from_secs(8)).await;

        let result = auth_client.verify_token::<TaskTokenClaim>(&token).await;
        assert_eq!(result.err().unwrap().to_string(), "JWT error: ExpiredSignature");
    }
}

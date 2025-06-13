use base64::{alphabet::STANDARD, engine::general_purpose, Engine};
use libp2p_identity::secp256k1::Keypair;
use prost::Message;
use tokio::{net::TcpStream, spawn};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use crate::{error::DiscoveryError, protobuf::common::Capability, utils::{handle_message, new_register_node_request_v1}};
use futures::{stream::SplitSink, StreamExt, SinkExt};

async fn setup_ws(url: &str, registration_credentials: &str) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, DiscoveryError> {
    use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest};

    let mut req = url.into_client_request().unwrap();
    req.headers_mut().insert("Authorization", format!("Basic {}", registration_credentials).parse().unwrap());

    let (socket, _) = connect_async(req).await?;
    Ok(socket)
}

pub struct DiscoClient {
    ws: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tokio_tungstenite::tungstenite::Message>,
    keypair: Keypair,
    centralized_id: String,
}

impl DiscoClient {
    pub async fn new(keypair: &Keypair, disco_url: &str, registration_credentials: &str) -> Result<Self, DiscoveryError> {
        let decoded = general_purpose::STANDARD.decode(registration_credentials).map_err(|_| DiscoveryError::InvalidCredentials)?;
        let decoded_str = String::from_utf8(decoded).map_err(|_| DiscoveryError::InvalidCredentials)?;
        let parts: Vec<&str> = decoded_str.split(':').collect();
        if parts.len() != 2 {
            return Err(DiscoveryError::InvalidCredentials);
        }
        let centralized_id = parts[0].to_string();
        
        let ws = setup_ws(disco_url, registration_credentials).await?;
        let (writer, mut reader) = ws.split();
        spawn(async move {
            while let Some(msg) = reader.next().await {
                match msg {
                    Ok(msg) => {
                        if msg.is_close() {
                            tracing::warn!("Disco connection closed: {:?}", msg);
                            continue;
                        }
                        if !msg.is_binary() {
                            tracing::error!("Message is expected to be in binary");
                            continue;
                        }
                        if let Err(e) = handle_message(&msg.into_data()) {
                            tracing::error!("Failed to parse msg: {:?}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to load message from ws: {:?}", e);
                    }
                }
            }
        });
        Ok(DiscoClient { ws: writer, keypair: keypair.clone(), centralized_id })
    }
    pub async fn register_compatible(&mut self, addrs: Vec<String>, capabilities: &[Capability]) -> Result<(), DiscoveryError> {
        let message = new_register_node_request_v1(&self.centralized_id, &self.keypair, addrs);
        let message = tokio_tungstenite::tungstenite::Message::binary(message.encode_to_vec());
        self.ws.send(message).await?;
        Ok(())
    }
}

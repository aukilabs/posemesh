use libp2p::{identity, PeerId};

use crate::{Error, Result};

/// One process-lifetime Ed25519 identity shared by DDS proof and libp2p Noise.
#[derive(Clone)]
pub struct Identity {
    keypair: identity::Keypair,
    peer_id: PeerId,
}

impl Identity {
    pub fn generate() -> Self {
        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();
        Self { keypair, peer_id }
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    pub fn public_key_protobuf(&self) -> Vec<u8> {
        self.keypair.public().encode_protobuf()
    }

    /// Signs the exact challenge bytes supplied by DDS without hashing,
    /// prefixing, or otherwise transforming them first.
    pub fn sign_challenge(&self, challenge: &[u8]) -> Result<Vec<u8>> {
        self.keypair
            .sign(challenge)
            .map_err(|error| Error::IdentitySigning(error.to_string()))
    }

    pub(crate) fn keypair(&self) -> identity::Keypair {
        self.keypair.clone()
    }
}

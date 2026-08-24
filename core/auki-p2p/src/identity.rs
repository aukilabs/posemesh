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

    /// Restore one canonical Ed25519 libp2p private key.
    ///
    /// The encoding is the cross-language protobuf format used by
    /// `libp2p-identity` and go-libp2p's `crypto.MarshalPrivateKey`. Unknown
    /// fields, non-canonical encodings, and other key algorithms are rejected.
    pub fn from_protobuf_encoding(bytes: &[u8]) -> Result<Self> {
        let keypair = identity::Keypair::from_protobuf_encoding(bytes)
            .map_err(|_| Error::InvalidIdentityPrivateKey)?;
        if keypair.key_type() != identity::KeyType::Ed25519 {
            return Err(Error::UnsupportedIdentityKeyType);
        }
        let canonical = keypair
            .to_protobuf_encoding()
            .map_err(|_| Error::InvalidIdentityPrivateKey)?;
        if canonical != bytes {
            return Err(Error::InvalidIdentityPrivateKey);
        }
        let peer_id = keypair.public().to_peer_id();
        Ok(Self { keypair, peer_id })
    }

    /// Export the canonical private-key encoding accepted by
    /// [`Identity::from_protobuf_encoding`]. Treat the returned bytes as a
    /// secret.
    pub fn to_protobuf_encoding(&self) -> Result<Vec<u8>> {
        self.keypair
            .to_protobuf_encoding()
            .map_err(|_| Error::InvalidIdentityPrivateKey)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protobuf_round_trip_preserves_peer_id_and_signing_identity() {
        let original = Identity::generate();
        let encoded = original.to_protobuf_encoding().unwrap();
        let restored = Identity::from_protobuf_encoding(&encoded).unwrap();

        assert_eq!(restored.peer_id(), original.peer_id());
        let challenge = b"persisted-p2p-identity";
        let signature = restored.sign_challenge(challenge).unwrap();
        assert!(original.keypair().public().verify(challenge, &signature));
    }

    #[test]
    fn malformed_and_noncanonical_private_keys_fail_closed() {
        assert!(matches!(
            Identity::from_protobuf_encoding(b"not-a-private-key"),
            Err(Error::InvalidIdentityPrivateKey)
        ));

        let identity = Identity::generate();
        let mut encoded = identity.to_protobuf_encoding().unwrap();
        encoded.extend_from_slice(&[0x18, 0x00]);
        assert!(matches!(
            Identity::from_protobuf_encoding(&encoded),
            Err(Error::InvalidIdentityPrivateKey)
        ));
    }
}

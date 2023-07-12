//! ED25519 signature verification

use ed25519_dalek::Signature;
use fuel_types::{
    Bytes32,
    Bytes64,
};

use crate::{
    Error,
    Message,
};

/// Verify a signature against a message digest and a public key.
pub fn verify(
    pub_key: &Bytes32,
    signature: &Bytes64,
    message: &Message,
) -> Result<(), Error> {
    let Ok(signature) = Signature::from_bytes(&**signature) else {
        return Err(Error::InvalidSignature);
    };

    let Ok(pub_key) = ed25519_dalek::PublicKey::from_bytes(&**pub_key) else {
        return Err(Error::InvalidPublicKey);
    };

    if pub_key.verify_strict(&**message, &signature).is_ok() {
        Ok(())
    } else {
        Err(Error::InvalidSignature)
    }
}

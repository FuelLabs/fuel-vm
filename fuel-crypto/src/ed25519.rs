//! ED25519 signature verification

use ed25519_dalek::Signature;
use fuel_types::{
    Bytes32,
    Bytes64,
};

use crate::Error;

/// Verify a signature against a message digest and a public key.
pub fn verify(
    pub_key: &Bytes32,
    signature: &Bytes64,
    message: &[u8],
) -> Result<(), Error> {
    let signature = Signature::from_bytes(signature);

    let pub_key = ed25519_dalek::VerifyingKey::from_bytes(pub_key)
        .map_err(|_| Error::InvalidPublicKey)?;

    if pub_key.verify_strict(message, &signature).is_ok() {
        Ok(())
    } else {
        Err(Error::InvalidSignature)
    }
}

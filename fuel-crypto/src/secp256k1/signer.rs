use super::{PublicKey, SecretKey, Signature};
use crate::{Error, Message};

use borrown::Borrown;

/// Signature provider based on a keystore
pub trait Signer_OLD {
    type Error: From<Error>;

    type KeyId;

    // /// Secret key indexed by `id`.
    // fn id_secret(&self, id: &Self::KeyId) -> Result<Borrown<'_, SecretKey>, Self::Error> {
    //     let keystore = self.keystore()?;
    //     let secret = keystore.secret(id)?.ok_or(Error::KeyNotFound)?;

    //     Ok(secret)
    // }

    // /// Public key indexed by `id`.
    // fn id_public(&self, id: &Self::KeyId) -> Result<Borrown<'_, PublicKey>, Self::Error> {
    //     let keystore = self.keystore()?;
    //     let public = keystore.public(id)?.ok_or(Error::KeyNotFound)?;

    //     Ok(public)
    // }

    // /// Sign a given message with the secret key identified by `id`
    // fn sign(&self, id: &Self::KeyId, message: &Message) -> Result<Signature, Self::Error> {
    //     let secret = self.id_secret(id)?;

    //     self.sign_with_key(secret.as_ref(), message)
    // }

    /// Sign a given message with the provided key
    #[cfg(not(feature = "std"))]
    fn sign_with_key(&self, secret: &SecretKey, message: &Message) -> Result<Signature, Self::Error>;

    /// Sign a given message with the provided key
    #[cfg(feature = "std")]
    fn sign_with_key(&self, secret: &SecretKey, message: &Message) -> Result<Signature, Self::Error> {
        Ok(Signature::sign(secret, message))
    }
}

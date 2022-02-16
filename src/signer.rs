use crate::{Keystore, Message, Signature};

/// Signature provider based on a keystore
pub trait Signer {
    /// Concrete keystore implementation
    type Keystore: Keystore;

    /// Accessor to the keystore
    fn keystore(&self) -> &Self::Keystore;

    /// Sign a given message with the secret key identified by `id`
    fn sign(
        &self,
        id: <Self::Keystore as Keystore>::KeyId,
        message: &Message,
    ) -> Result<Signature, <Self::Keystore as Keystore>::Error> {
        self.keystore()
            .secret(id)
            .map(|secret| Signature::sign(secret.as_ref(), message))
    }

    /// Verify a given message with the public key identified by `id`
    fn verify(
        &self,
        id: <Self::Keystore as Keystore>::KeyId,
        signature: Signature,
        message: &Message,
    ) -> Result<(), <Self::Keystore as Keystore>::Error> {
        let public = self.keystore().public(id)?;

        signature
            .verify(public.as_ref(), message)
            .map_err(|e| e.into())
    }
}

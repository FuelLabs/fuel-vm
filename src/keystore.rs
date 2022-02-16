use crate::{Error, PublicKey, SecretKey};

use borrown::Borrown;

/// Keys container
pub trait Keystore {
    /// Keystore error implementation
    type Error: From<Error>;

    /// Identifier for the keypair
    type KeyId;

    /// Public key for a given id
    fn public(&self, id: Self::KeyId) -> Result<Borrown<'_, PublicKey>, Self::Error>;

    /// Secret key for a given id
    fn secret(&self, id: Self::KeyId) -> Result<Borrown<'_, SecretKey>, Self::Error>;
}

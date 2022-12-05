use crate::{Error, PublicKey, SecretKey};

use borrown::Borrown;

/// Keys container
pub trait Keystore {
    /// Keystore error implementation
    type Error: From<Error>;

    /// Identifier for the keypair
    type KeyId;

    /// Secret key for a given id
    fn secret(&self, id: &Self::KeyId) -> Result<Option<Borrown<'_, SecretKey>>, Self::Error>;

    /// Public key for a given id
    #[cfg(not(feature = "std"))]
    fn public(&self, id: &Self::KeyId) -> Result<Option<Borrown<'_, PublicKey>>, Self::Error>;

    /// Public key for a given id
    #[cfg(feature = "std")]
    fn public(&self, id: &Self::KeyId) -> Result<Option<Borrown<'_, PublicKey>>, Self::Error> {
        let secret = self.secret(id)?;
        let public = secret
            .map(|s| PublicKey::from(s.as_ref()))
            .map(Borrown::Owned);

        Ok(public)
    }
}

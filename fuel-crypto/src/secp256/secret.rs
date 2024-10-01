use fuel_types::Bytes32;

use core::{
    fmt,
    ops::Deref,
    str,
};

use zeroize::Zeroize;

use crate::{
    secp256::PublicKey,
    Error,
};

#[cfg(feature = "std")]
use coins_bip32::path::DerivationPath;

#[cfg(feature = "std")]
use coins_bip39::{
    English,
    Mnemonic,
};

#[cfg(feature = "random")]
use rand::{
    CryptoRng,
    RngCore,
};

/// Asymmetric secret key, guaranteed to be valid by construction
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct SecretKey(Bytes32);

impl SecretKey {
    /// Memory length of the type
    pub const LEN: usize = Bytes32::LEN;
}

impl Deref for SecretKey {
    type Target = [u8; SecretKey::LEN];

    fn deref(&self) -> &[u8; SecretKey::LEN] {
        self.0.deref()
    }
}

impl AsRef<[u8]> for SecretKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<SecretKey> for [u8; SecretKey::LEN] {
    fn from(salt: SecretKey) -> [u8; SecretKey::LEN] {
        salt.0.into()
    }
}

impl fmt::LowerHex for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::UpperHex for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<::k256::SecretKey> for SecretKey {
    fn from(s: ::k256::SecretKey) -> Self {
        let mut raw_bytes = [0u8; Self::LEN];
        raw_bytes.copy_from_slice(&s.to_bytes());
        Self(Bytes32::from(raw_bytes))
    }
}

#[cfg(feature = "std")]
impl From<::secp256k1::SecretKey> for SecretKey {
    fn from(s: ::secp256k1::SecretKey) -> Self {
        let mut raw_bytes = [0u8; Self::LEN];
        raw_bytes.copy_from_slice(s.as_ref());
        Self(Bytes32::from(raw_bytes))
    }
}

impl From<&SecretKey> for ::k256::SecretKey {
    fn from(sk: &SecretKey) -> Self {
        ::k256::SecretKey::from_bytes(&(*sk.0).into())
            .expect("SecretKey is guaranteed to be valid")
    }
}

#[cfg(feature = "std")]
impl From<&SecretKey> for ::secp256k1::SecretKey {
    fn from(sk: &SecretKey) -> Self {
        ::secp256k1::SecretKey::from_slice(sk.as_ref())
            .expect("SecretKey is guaranteed to be valid")
    }
}

#[cfg(all(feature = "random", feature = "test-helpers"))]
impl Default for SecretKey {
    /// Creates a new random secret using rand::thread_rng()
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        SecretKey::random(&mut rng)
    }
}

#[cfg(feature = "std")]
pub type W = English;

impl SecretKey {
    /// Create a new random secret
    #[cfg(feature = "random")]
    pub fn random(rng: &mut (impl CryptoRng + RngCore)) -> Self {
        super::backend::k1::random_secret(rng)
    }

    /// Generate a new secret key from a mnemonic phrase and its derivation path.
    /// Both are passed as `&str`. If you want to manually create a `DerivationPath`
    /// and `Mnemonic`, use [`SecretKey::new_from_mnemonic`].
    /// The derivation path is a list of integers, each representing a child index.
    #[cfg(feature = "std")]
    pub fn new_from_mnemonic_phrase_with_path(
        phrase: &str,
        path: &str,
    ) -> Result<Self, Error> {
        use core::str::FromStr;

        let mnemonic = Mnemonic::<W>::new_from_phrase(phrase)?;
        let path = DerivationPath::from_str(path)?;
        Self::new_from_mnemonic(path, mnemonic)
    }

    /// Generate a new secret key from a `DerivationPath` and `Mnemonic`.
    /// If you want to pass strings instead, use
    /// [`SecretKey::new_from_mnemonic_phrase_with_path`].
    #[cfg(feature = "std")]
    pub fn new_from_mnemonic(d: DerivationPath, m: Mnemonic<W>) -> Result<Self, Error> {
        let derived_priv_key = m.derive_key(d, None)?;
        let key: &coins_bip32::prelude::SigningKey = derived_priv_key.as_ref();
        let bytes: [u8; Self::LEN] = key.to_bytes().into();
        Ok(SecretKey(Bytes32::from(bytes)))
    }

    /// Return the curve representation of this secret.
    pub fn public_key(&self) -> PublicKey {
        crate::secp256::backend::k1::public_key(self)
    }
}

impl TryFrom<Bytes32> for SecretKey {
    type Error = Error;

    fn try_from(b: Bytes32) -> Result<Self, Self::Error> {
        match k256::SecretKey::from_bytes((&*b).into()) {
            Ok(_) => Ok(Self(b)),
            Err(_) => Err(Error::InvalidSecretKey),
        }
    }
}

impl TryFrom<&[u8]> for SecretKey {
    type Error = Error;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        Bytes32::try_from(slice)
            .map_err(|_| Error::InvalidSecretKey)
            .and_then(SecretKey::try_from)
    }
}

impl str::FromStr for SecretKey {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Bytes32::from_str(s)
            .map_err(|_| Error::InvalidSecretKey)
            .and_then(SecretKey::try_from)
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    #[cfg(feature = "random")]
    #[test]
    fn default__yields_valid_secret() {
        use super::SecretKey;
        let _ = SecretKey::default();
    }
}

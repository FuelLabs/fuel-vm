use fuel_types::Bytes32;

use core::{
    fmt,
    ops::Deref,
    str,
};

use zeroize::Zeroize;

use crate::{
    Error,
    PublicKey,
};
use coins_bip32::path::DerivationPath;
use coins_bip39::{
    English,
    Mnemonic,
};
use std::str::FromStr;

#[cfg(feature = "random")]
use rand::{
    CryptoRng,
    RngCore,
};

/// Asymmetric secret key
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct SecretKey(Bytes32);

impl SecretKey {
    /// Memory length of the type
    pub const LEN: usize = Bytes32::LEN;

    /// Construct a `SecretKey` directly from its bytes.
    ///
    /// This constructor expects the given bytes to be a valid secret key. Validity is
    /// unchecked.
    fn from_bytes_unchecked(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes.into())
    }
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

impl From<k256::SecretKey> for SecretKey {
    fn from(s: k256::SecretKey) -> Self {
        let mut raw_bytes = [0u8; Self::LEN];
        raw_bytes.copy_from_slice(&s.to_bytes());
        Self(Bytes32::from(raw_bytes))
    }
}

impl Into<k256::SecretKey> for SecretKey {
    fn into(self) -> k256::SecretKey {
        k256::SecretKey::from_bytes((&*self.0).into()).expect("Invalid secret key")
    }
}

pub type W = English;

impl SecretKey {
    /// Create a new random secret
    #[cfg(feature = "random")]
    pub fn random(rng: &mut (impl CryptoRng + RngCore)) -> Self {
        k256::SecretKey::random(rng).into()
    }

    /// Generate a new secret key from a mnemonic phrase and its derivation path.
    /// Both are passed as `&str`. If you want to manually create a `DerivationPath`
    /// and `Mnemonic`, use [`SecretKey::new_from_mnemonic`].
    /// The derivation path is a list of integers, each representing a child index.
    pub fn new_from_mnemonic_phrase_with_path(
        phrase: &str,
        path: &str,
    ) -> Result<Self, Error> {
        let mnemonic = Mnemonic::<W>::new_from_phrase(phrase)?;
        let path = DerivationPath::from_str(path)?;
        Self::new_from_mnemonic(path, mnemonic)
    }

    /// Generate a new secret key from a `DerivationPath` and `Mnemonic`.
    /// If you want to pass strings instead, use
    /// [`SecretKey::new_from_mnemonic_phrase_with_path`].
    pub fn new_from_mnemonic(d: DerivationPath, m: Mnemonic<W>) -> Result<Self, Error> {
        let derived_priv_key = m.derive_key(d, None)?;
        let key: &coins_bip32::prelude::SigningKey = derived_priv_key.as_ref();
        let bytes: [u8; Self::LEN] = key.to_bytes().into();
        Ok(SecretKey::from_bytes_unchecked(bytes))
    }

    /// Return the curve representation of this secret.
    ///
    /// The discrete logarithm property guarantees this is a one-way
    /// function.
    pub fn public_key(&self) -> PublicKey {
        let vk: k256::SecretKey = (*self).into();
        vk.public_key().into()
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

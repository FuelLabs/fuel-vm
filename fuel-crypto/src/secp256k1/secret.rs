use fuel_types::Bytes32;

use core::{
    fmt,
    ops::Deref,
};

use zeroize::Zeroize;

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
    #[cfg(feature = "std")]
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

#[cfg(feature = "std")]
mod use_std {
    use super::*;
    use crate::{
        Error,
        PublicKey,
    };
    use coins_bip32::path::DerivationPath;
    use coins_bip39::{
        English,
        Mnemonic,
    };
    use core::{
        borrow::Borrow,
        str,
    };
    use k256::{
        Error as Secp256k1Error,
        SecretKey as Secp256k1SecretKey,
    };
    use std::str::FromStr;

    #[cfg(feature = "random")]
    use rand::{
        distributions::{
            Distribution,
            Standard,
        },
        Rng,
    };

    pub type W = English;

    impl SecretKey {
        /// Create a new random secret
        #[cfg(feature = "random")]
        pub fn random<R>(rng: &mut R) -> Self
        where
            R: rand::Rng + ?Sized,
        {
            // TODO there is no clear API to generate a scalar for secp256k1. This code is
            // very inefficient and not constant time; it was copied from
            // https://github.com/rust-bitcoin/rust-secp256k1/blob/ada3f98ab65e6f12cf1550edb0b7ae064ecac153/src/key.rs#L101
            //
            // Need to improve; generate random bytes and truncate to the field.
            //
            // We don't call `Secp256k1SecretKey::new` here because the `rand`
            // requirements are outdated and inconsistent.
            let mut secret = Bytes32::zeroed();
            loop {
                rng.fill(secret.as_mut());
                if secret_key_bytes_valid(&secret).is_ok() {
                    break
                }
            }
            Self(secret)
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
        pub fn new_from_mnemonic(
            d: DerivationPath,
            m: Mnemonic<W>,
        ) -> Result<Self, Error> {
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
            PublicKey::from(self)
        }
    }

    impl TryFrom<Bytes32> for SecretKey {
        type Error = Error;

        fn try_from(b: Bytes32) -> Result<Self, Self::Error> {
            secret_key_bytes_valid(&b).map(|_| Self(b))
        }
    }

    impl TryFrom<&[u8]> for SecretKey {
        type Error = Error;

        fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
            Bytes32::try_from(slice)
                .map_err(|_| Secp256k1Error::InvalidSecretKey.into())
                .and_then(SecretKey::try_from)
        }
    }

    impl str::FromStr for SecretKey {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Bytes32::from_str(s)
                .map_err(|_| Secp256k1Error::InvalidSecretKey.into())
                .and_then(SecretKey::try_from)
        }
    }

    impl Borrow<Secp256k1SecretKey> for SecretKey {
        fn borrow(&self) -> &Secp256k1SecretKey {
            // Safety: field checked. The memory representation of the secp256k1 key is
            // `[u8; 32]`
            #[allow(unsafe_code)]
            unsafe {
                &*(self.as_ref().as_ptr() as *const Secp256k1SecretKey)
            }
        }
    }

    #[cfg(feature = "random")]
    impl rand::Fill for SecretKey {
        fn try_fill<R: rand::Rng + ?Sized>(
            &mut self,
            rng: &mut R,
        ) -> Result<(), rand::Error> {
            *self = Self::random(rng);

            Ok(())
        }
    }

    #[cfg(feature = "random")]
    impl Distribution<SecretKey> for Standard {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SecretKey {
            SecretKey::random(rng)
        }
    }

    /// Check if the secret key byte representation is within the curve.
    fn secret_key_bytes_valid(bytes: &[u8; SecretKey::LEN]) -> Result<(), Error> {
        secp256k1::SecretKey::from_slice(bytes)
            .map(|_| ())
            .map_err(Into::into)
    }
}

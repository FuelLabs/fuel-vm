use fuel_types::Bytes32;

use core::fmt;
use core::ops::Deref;

/// Asymmetric secret key
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct SecretKey(Bytes32);

impl SecretKey {
    /// Memory length of the type
    pub const LEN: usize = Bytes32::LEN;

    /// Add a conversion from arbitrary slices into owned
    ///
    /// # Safety
    ///
    /// There is no guarantee the provided bytes will fit the field. The field
    /// security can be checked with [`SecretKey::is_in_field`].
    pub unsafe fn from_bytes_unchecked(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes.into())
    }

    /// Add a conversion from arbitrary slices into owned
    ///
    /// # Safety
    ///
    /// This function will not panic if the length of the slice is smaller than
    /// `Self::LEN`. Instead, it will cause undefined behavior and read random
    /// disowned bytes.
    ///
    /// There is no guarantee the provided bytes will fit the field.
    pub unsafe fn from_slice_unchecked(bytes: &[u8]) -> Self {
        Self(Bytes32::from_slice_unchecked(bytes))
    }

    /// Copy-free reference cast
    ///
    /// There is no guarantee the provided bytes will fit the field.
    ///
    /// # Safety
    ///
    /// Inputs smaller than `Self::LEN` will cause undefined behavior.
    pub unsafe fn as_ref_unchecked(bytes: &[u8]) -> &Self {
        // The interpreter will frequently make references to keys and values using
        // logically checked slices.
        //
        // This function will avoid unnecessary copy to owned slices for the interpreter
        // access
        &*(bytes.as_ptr() as *const Self)
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
    use crate::{Error, PublicKey};

    use secp256k1::{Error as Secp256k1Error, SecretKey as Secp256k1SecretKey};

    use core::borrow::Borrow;
    use core::str;

    #[cfg(feature = "random")]
    use rand::{
        distributions::{Distribution, Standard},
        Rng,
    };

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
            // We don't call `Secp256k1SecretKey::new` here because the `rand` requirements
            // are outdated and inconsistent.

            use secp256k1::ffi::{self, CPtr};

            let mut secret = Bytes32::zeroed();

            loop {
                rng.fill(secret.as_mut());

                // Safety: FFI call
                let overflow = unsafe {
                    ffi::secp256k1_ec_seckey_verify(
                        ffi::secp256k1_context_no_precomp,
                        secret.as_c_ptr(),
                    )
                };

                if overflow != 0 {
                    break;
                }
            }

            Self(secret)
        }

        /// Check if the provided slice represents a scalar that fits the field.
        ///
        /// # Safety
        ///
        /// This function extends the unsafety of
        /// [`SecretKey::as_ref_unchecked`].
        pub unsafe fn is_slice_in_field_unchecked(slice: &[u8]) -> bool {
            use secp256k1::ffi::{self, CPtr};

            let secret = Self::as_ref_unchecked(slice);

            // Safety: FFI call
            let overflow = ffi::secp256k1_ec_seckey_verify(
                ffi::secp256k1_context_no_precomp,
                secret.as_c_ptr(),
            );

            overflow != 0
        }

        /// Check if the secret key representation fits the scalar field.
        pub fn is_in_field(&self) -> bool {
            // Safety: struct is guaranteed to reference itself with correct len
            unsafe { Self::is_slice_in_field_unchecked(self.as_ref()) }
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
            let secret = SecretKey(b);

            secret
                .is_in_field()
                .then(|| secret)
                .ok_or(Error::InvalidSecretKey)
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
            unsafe { &*(self.as_ref().as_ptr() as *const Secp256k1SecretKey) }
        }
    }

    #[cfg(feature = "random")]
    impl rand::Fill for SecretKey {
        fn try_fill<R: rand::Rng + ?Sized>(&mut self, rng: &mut R) -> Result<(), rand::Error> {
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
}

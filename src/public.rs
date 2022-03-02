use crate::Hasher;

use fuel_types::{Bytes32, Bytes64};

use core::fmt;
use core::ops::Deref;

/// Asymmetric public key
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// TODO serde implementation blocked by https://github.com/FuelLabs/fuel-types/issues/13
pub struct PublicKey(Bytes64);

impl PublicKey {
    /// Memory length of the type
    pub const LEN: usize = Bytes64::LEN;

    /// Copy-free reference cast
    ///
    /// # Safety
    ///
    /// This function will not panic if the length of the slice is smaller than
    /// `Self::LEN`. Instead, it will cause undefined behavior and read random
    /// disowned bytes.
    ///
    /// There is no guarantee the provided bytes will fit the curve.
    pub unsafe fn as_ref_unchecked(bytes: &[u8]) -> &Self {
        // The interpreter will frequently make references to keys and values using
        // logically checked slices.
        //
        // This function will save unnecessary copy to owned slices for the interpreter
        // access
        &*(bytes.as_ptr() as *const Self)
    }

    /// Add a conversion from arbitrary slices into owned
    ///
    /// # Safety
    ///
    /// There is no guarantee the provided bytes will fit the curve. The curve
    /// security can be checked with [`PublicKey::is_in_curve`].
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
    /// There is no guarantee the provided bytes will fit the curve.
    pub unsafe fn from_slice_unchecked(bytes: &[u8]) -> Self {
        Self(Bytes64::from_slice_unchecked(bytes))
    }

    /// Hash of the public key
    pub fn hash(&self) -> Bytes32 {
        Hasher::hash(self.as_ref())
    }
}

impl Deref for PublicKey {
    type Target = [u8; PublicKey::LEN];

    fn deref(&self) -> &[u8; PublicKey::LEN] {
        self.0.deref()
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsMut<[u8]> for PublicKey {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

impl From<PublicKey> for [u8; PublicKey::LEN] {
    fn from(pk: PublicKey) -> [u8; PublicKey::LEN] {
        pk.0.into()
    }
}

impl fmt::LowerHex for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::UpperHex for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "std")]
mod use_std {
    use super::*;
    use crate::{Error, SecretKey};

    use secp256k1::{Error as Secp256k1Error, PublicKey as Secp256k1PublicKey, Secp256k1};

    use core::borrow::Borrow;
    use core::str;

    const UNCOMPRESSED_PUBLIC_KEY_SIZE: usize = 65;

    // Internal secp256k1 identifier for uncompressed point
    //
    // https://github.com/rust-bitcoin/rust-secp256k1/blob/ecb62612b57bf3aa8d8017d611d571f86bfdb5dd/secp256k1-sys/depend/secp256k1/include/secp256k1.h#L196
    const SECP_UNCOMPRESSED_FLAG: u8 = 4;

    impl PublicKey {
        /// Check if the provided slice represents a public key that is in the
        /// curve.
        ///
        /// # Safety
        ///
        /// This function extends the unsafety of
        /// [`PublicKey::as_ref_unchecked`].
        pub unsafe fn is_slice_in_curve_unchecked(slice: &[u8]) -> bool {
            use secp256k1::ffi::{self, CPtr};

            let public = Self::as_ref_unchecked(slice);

            let mut public_with_flag = [0u8; UNCOMPRESSED_PUBLIC_KEY_SIZE];

            public_with_flag[1..].copy_from_slice(public.as_ref());

            // Safety: FFI call
            let curve = ffi::secp256k1_ec_pubkey_parse(
                ffi::secp256k1_context_no_precomp,
                &mut ffi::PublicKey::new(),
                public_with_flag.as_c_ptr(),
                UNCOMPRESSED_PUBLIC_KEY_SIZE,
            );

            curve == 1
        }

        /// Check if the secret key representation is in the curve.
        pub fn is_in_curve(&self) -> bool {
            // Safety: struct is guaranteed to reference itself with correct len
            unsafe { Self::is_slice_in_curve_unchecked(self.as_ref()) }
        }

        pub(crate) fn from_secp(pk: &Secp256k1PublicKey) -> PublicKey {
            debug_assert_eq!(
                UNCOMPRESSED_PUBLIC_KEY_SIZE,
                secp256k1::constants::UNCOMPRESSED_PUBLIC_KEY_SIZE
            );

            let pk = pk.serialize_uncompressed();

            debug_assert_eq!(SECP_UNCOMPRESSED_FLAG, pk[0]);

            // Ignore the first byte of the compression flag
            let pk = &pk[1..];

            // Safety: compile-time assertion of size correctness
            unsafe { Self::from_slice_unchecked(pk) }
        }

        pub(crate) fn _to_secp(&self) -> Result<Secp256k1PublicKey, Error> {
            let mut pk = [SECP_UNCOMPRESSED_FLAG; UNCOMPRESSED_PUBLIC_KEY_SIZE];

            debug_assert_eq!(SECP_UNCOMPRESSED_FLAG, pk[0]);

            pk[1..].copy_from_slice(self.as_ref());

            let pk = Secp256k1PublicKey::from_slice(&pk)?;

            Ok(pk)
        }
    }

    impl TryFrom<Bytes64> for PublicKey {
        type Error = Error;

        fn try_from(b: Bytes64) -> Result<Self, Self::Error> {
            let public = PublicKey(b);

            public
                .is_in_curve()
                .then(|| public)
                .ok_or(Error::InvalidPublicKey)
        }
    }

    impl TryFrom<&[u8]> for PublicKey {
        type Error = Error;

        fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
            Bytes64::try_from(slice)
                .map_err(|_| Secp256k1Error::InvalidPublicKey.into())
                .and_then(PublicKey::try_from)
        }
    }

    impl From<&SecretKey> for PublicKey {
        fn from(s: &SecretKey) -> PublicKey {
            let secp = Secp256k1::new();

            let secret = s.borrow();

            // Copy here is unavoidable since there is no API in secp256k1 to create
            // uncompressed keys directly
            let public = Secp256k1PublicKey::from_secret_key(&secp, secret);

            Self::from_secp(&public)
        }
    }

    impl str::FromStr for PublicKey {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Bytes64::from_str(s)
                .map_err(|_| Secp256k1Error::InvalidPublicKey.into())
                .and_then(PublicKey::try_from)
        }
    }
}

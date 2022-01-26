use crate::Hasher;

use fuel_types::{Bytes32, Bytes64};

use core::fmt;
use core::ops::Deref;

/// Signature public key
///
/// The compression scheme is described in
/// <https://github.com/lazyledger/lazyledger-specs/blob/master/specs/data_structures.md#public-key-cryptography>
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

impl From<PublicKey> for [u8; PublicKey::LEN] {
    fn from(salt: PublicKey) -> [u8; PublicKey::LEN] {
        salt.0.into()
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
    use crate::{Error, SecretKey, Signature};

    use secp256k1::{
        recovery::{RecoverableSignature, RecoveryId},
        {Error as Secp256k1Error, Message, PublicKey as Secp256k1PublicKey, Secp256k1},
    };

    use core::borrow::Borrow;
    use core::str;

    const UNCOMPRESSED_PUBLIC_KEY_SIZE: usize = 65;

    impl PublicKey {
        /// Convert an uncompressed public key representation into self.
        ///
        /// # Safety
        ///
        /// Will not check elliptic-curve correctness.
        pub unsafe fn from_uncompressed_unchecked(
            pk: [u8; UNCOMPRESSED_PUBLIC_KEY_SIZE],
        ) -> PublicKey {
            debug_assert_eq!(
                UNCOMPRESSED_PUBLIC_KEY_SIZE,
                secp256k1::constants::UNCOMPRESSED_PUBLIC_KEY_SIZE
            );

            // Ignore the first byte of the compressed flag
            let pk = &pk[1..];

            // Safety: compile-time assertion of size correctness
            Self::from_slice_unchecked(pk)
        }

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

            (&mut public_with_flag[1..]).copy_from_slice(public.as_ref());

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

        /// Recover the public key from a signature performed with
        /// [`SecretKey::sign`]
        pub fn recover<M>(signature: Signature, message: M) -> Result<PublicKey, Error>
        where
            M: AsRef<[u8]>,
        {
            let message = SecretKey::normalize_message(message);

            Self::_recover(signature, &message)
        }

        /// Recover the public key from a signature performed with
        /// [`SecretKey::sign`]
        ///
        /// # Safety
        ///
        /// The protocol expects the message to be the result of a hash -
        /// otherwise, its verification is malleable. The output of the
        /// hash must be 32 bytes.
        ///
        /// The unsafe directive of this function is related only to the message
        /// input. It might fail if the signature is inconsistent.
        pub unsafe fn recover_unchecked<M>(
            signature: Signature,
            message: M,
        ) -> Result<PublicKey, Error>
        where
            M: AsRef<[u8]>,
        {
            let message = SecretKey::cast_message(message.as_ref());

            Self::_recover(signature, message)
        }

        fn _recover(mut signature: Signature, message: &Message) -> Result<PublicKey, Error> {
            let v = ((signature.as_mut()[32] & 0x90) >> 7) as i32;
            signature.as_mut()[32] &= 0x7f;

            let v = RecoveryId::from_i32(v)?;
            let signature = RecoverableSignature::from_compact(signature.as_ref(), v)?;

            let pk = Secp256k1::new()
                .recover(message, &signature)?
                .serialize_uncompressed();

            // Ignore the first byte of the compressed flag
            let pk = &pk[1..];

            // Safety: secp256k1 protocol specifies 65 bytes output
            let pk = unsafe { Bytes64::from_slice_unchecked(pk) };

            Ok(Self(pk))
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
            let public =
                Secp256k1PublicKey::from_secret_key(&secp, secret).serialize_uncompressed();

            // Safety: FFI is guaranteed to return valid public key.
            unsafe { PublicKey::from_uncompressed_unchecked(public) }
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

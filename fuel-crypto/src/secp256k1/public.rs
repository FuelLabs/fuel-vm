use crate::hasher::Hasher;
use core::{
    fmt,
    ops::Deref,
};
use fuel_types::{
    Bytes32,
    Bytes64,
};

/// Asymmetric secp256k1 public key
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct PublicKey(Bytes64);

impl PublicKey {
    /// Memory length of the type in bytes.
    pub const LEN: usize = Bytes64::LEN;

    /// Construct a `PublicKey` directly from its bytes.
    ///
    /// This constructor expects the given bytes to be a valid public key, and
    /// does not check whether the public key is within the curve.
    #[cfg(feature = "std")]
    fn from_bytes_unchecked(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes.into())
    }

    /// Cryptographic hash of the public key.
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
    use crate::{
        Error,
        SecretKey,
    };

    use k256::{
        constants::UNCOMPRESSED_PUBLIC_KEY_SIZE,
        Error as Secp256k1Error,
        PublicKey as Secp256k1PublicKey,
        Secp256k1,
    };

    use core::{
        borrow::Borrow,
        str,
    };

    // Internal secp256k1 identifier for uncompressed point
    //
    // https://github.com/rust-bitcoin/rust-secp256k1/blob/ecb62612b57bf3aa8d8017d611d571f86bfdb5dd/secp256k1-sys/depend/secp256k1/include/secp256k1.h#L196
    const SECP_UNCOMPRESSED_FLAG: u8 = 4;

    impl PublicKey {
        pub(crate) fn from_secp(pk: &Secp256k1PublicKey) -> PublicKey {
            debug_assert_eq!(
                UNCOMPRESSED_PUBLIC_KEY_SIZE,
                secp256k1::constants::UNCOMPRESSED_PUBLIC_KEY_SIZE
            );

            let pk = pk.serialize_uncompressed();

            debug_assert_eq!(SECP_UNCOMPRESSED_FLAG, pk[0]);

            // Ignore the first byte of the compression flag
            let bytes = <[u8; Self::LEN]>::try_from(&pk[1..])
                .expect("compile-time bounds-checks");

            Self::from_bytes_unchecked(bytes)
        }

        pub(crate) fn to_secp(&self) -> Result<Secp256k1PublicKey, Error> {
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
            public_key_bytes_valid(&b).map(|_| Self(b))
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

    /// Check if the public key byte representation is in the curve.
    fn public_key_bytes_valid(bytes: &[u8; PublicKey::LEN]) -> Result<(), Error> {
        let mut public_with_flag = [0u8; UNCOMPRESSED_PUBLIC_KEY_SIZE];
        public_with_flag[1..].copy_from_slice(bytes);
        secp256k1::PublicKey::from_slice(&public_with_flag)
            .map(|_| ())
            .map_err(|_| Error::InvalidPublicKey)
    }
}

use super::backend::k1;
use crate::{
    Error,
    Message,
    PublicKey,
    SecretKey,
};

use fuel_types::Bytes64;

use core::{
    fmt,
    ops::Deref,
    str,
};

/// Compressed-form Secp256k1 signature.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct Signature(Bytes64);

impl Signature {
    /// Memory length of the type in bytes.
    pub const LEN: usize = Bytes64::LEN;

    /// Construct a `Signature` directly from its bytes.
    ///
    /// This constructor expects the given bytes to be a valid signature. No signing is
    /// performed.
    pub fn from_bytes(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes.into())
    }

    /// Construct a `Signature` reference directly from a reference to its bytes.
    ///
    /// This constructor expects the given bytes to be a valid signature. No signing is
    /// performed.
    pub fn from_bytes_ref(bytes: &[u8; Self::LEN]) -> &Self {
        // TODO: Wrap this unsafe conversion safely in `fuel_types::Bytes64`.
        #[allow(unsafe_code)]
        unsafe {
            &*(bytes.as_ptr() as *const Self)
        }
    }

    /// Kept temporarily for backwards compatibility.
    #[deprecated = "Use `Signature::from_bytes` instead"]
    pub fn from_bytes_unchecked(bytes: [u8; Self::LEN]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl Deref for Signature {
    type Target = [u8; Signature::LEN];

    fn deref(&self) -> &[u8; Signature::LEN] {
        self.0.deref()
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsMut<[u8]> for Signature {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

impl fmt::LowerHex for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::UpperHex for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Signature> for [u8; Signature::LEN] {
    fn from(salt: Signature) -> [u8; Signature::LEN] {
        salt.0.into()
    }
}

impl From<Signature> for Bytes64 {
    fn from(s: Signature) -> Self {
        s.0
    }
}

impl str::FromStr for Signature {
    type Err = Error;

    /// Parse a `Signature` directly from its bytes encoded as hex in a string.
    ///
    /// This constructor does not perform any signing.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Bytes64::from_str(s)
            .map_err(|_| Error::InvalidSignature)
            .map(|s| Self::from_bytes(s.into()))
    }
}

/// Secp256k1 methods
impl Signature {
    /// Produce secp256k1 signature
    pub fn sign(secret: &SecretKey, message: &Message) -> Self {
        Self(Bytes64::from(k1::sign(secret, message)))
    }

    /// Recover secp256k1 public key from a signature performed with
    pub fn recover(&self, message: &Message) -> Result<PublicKey, Error> {
        k1::recover(*self.0, message)
    }

    /// Verify that a signature matches given public key
    pub fn verify(&self, public_key: &PublicKey, message: &Message) -> Result<(), Error> {
        k1::verify(*self.0, **public_key, message)
    }
}

use crate::{
    Error,
    hasher::Hasher,
    secp256::SecretKey,
};
use core::{
    fmt,
    ops::Deref,
};

use k256::ecdsa::VerifyingKey;

use core::str;

use fuel_types::{
    Bytes32,
    Bytes64,
};
use k256::elliptic_curve::sec1::ToEncodedPoint;

/// Asymmetric secp256k1 public key, i.e. verifying key, in uncompressed form.
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic-primitives.md#ecdsa-public-key-cryptography>
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct PublicKey(Bytes64);

impl PublicKey {
    /// Memory length of the type in bytes.
    pub const LEN: usize = Bytes64::LEN;

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

impl From<k256::PublicKey> for PublicKey {
    fn from(key: k256::PublicKey) -> Self {
        let point = key.to_encoded_point(false);
        let mut raw = Bytes64::zeroed();
        raw[..32].copy_from_slice(point.x().unwrap());
        raw[32..].copy_from_slice(point.y().unwrap());
        Self(raw)
    }
}

impl From<&ecdsa::VerifyingKey<k256::Secp256k1>> for PublicKey {
    fn from(vk: &ecdsa::VerifyingKey<k256::Secp256k1>) -> Self {
        let vk: k256::PublicKey = vk.into();
        vk.into()
    }
}

#[cfg(feature = "std")]
impl From<secp256k1::PublicKey> for PublicKey {
    fn from(key: secp256k1::PublicKey) -> Self {
        let key_bytes = key.serialize_uncompressed();
        let mut raw = Bytes64::zeroed();
        // Remove leading identifier byte
        raw.copy_from_slice(&key_bytes[1..]);
        Self(raw)
    }
}

impl TryFrom<Bytes64> for PublicKey {
    type Error = Error;

    fn try_from(b: Bytes64) -> Result<Self, Self::Error> {
        match VerifyingKey::from_sec1_bytes(&*b) {
            Ok(_) => Ok(Self(b)),
            Err(_) => Err(Error::InvalidPublicKey),
        }
    }
}

impl TryFrom<&[u8]> for PublicKey {
    type Error = Error;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        Bytes64::try_from(slice)
            .map_err(|_| Error::InvalidPublicKey)
            .and_then(PublicKey::try_from)
    }
}

impl From<&SecretKey> for PublicKey {
    fn from(s: &SecretKey) -> PublicKey {
        s.public_key()
    }
}

impl str::FromStr for PublicKey {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Bytes64::from_str(s)
            .map_err(|_| Error::InvalidPublicKey)
            .and_then(PublicKey::try_from)
    }
}

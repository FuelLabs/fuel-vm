use core::ops::{
    Deref,
    DerefMut,
};
use educe::Educe;

use alloc::vec::Vec;
use fuel_types::bytes::Bytes;

#[derive(Clone, Default, Educe, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[educe(Eq, PartialEq, Hash, Debug)]
pub struct PredicateCode {
    pub bytes: Bytes,
}

impl PredicateCode {
    pub const fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Bytes::new(bytes),
        }
    }
}

impl From<Vec<u8>> for PredicateCode {
    fn from(bytes: Vec<u8>) -> Self {
        Self {
            bytes: bytes.into(),
        }
    }
}

impl From<&[u8]> for PredicateCode {
    fn from(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec().into(),
        }
    }
}

impl AsRef<[u8]> for PredicateCode {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl AsMut<[u8]> for PredicateCode {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
}

impl Deref for PredicateCode {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl DerefMut for PredicateCode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes
    }
}

#[cfg(feature = "da-compression")]
impl fuel_compression::Compressible for PredicateCode {
    type Compressed = fuel_compression::RegistryKey;
}

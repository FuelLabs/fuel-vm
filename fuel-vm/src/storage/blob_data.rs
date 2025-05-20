use core::borrow::Borrow;

use fuel_storage::Mappable;
use fuel_types::{
    BlobId,
    fmt_truncated_hex,
};

use alloc::vec::Vec;
use educe::Educe;

#[cfg(feature = "random")]
use rand::{
    Rng,
    distributions::{
        Distribution,
        Standard,
    },
};

/// The storage table for blob data bytes.
pub struct BlobData;

impl Mappable for BlobData {
    type Key = Self::OwnedKey;
    type OwnedKey = BlobId;
    type OwnedValue = BlobBytes;
    type Value = [u8];
}

/// Storage type for blob bytes
#[derive(Educe, Clone, PartialEq, Eq, Hash)]
#[educe(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlobBytes(#[educe(Debug(method(fmt_truncated_hex::<16>)))] pub Vec<u8>);

impl From<Vec<u8>> for BlobBytes {
    fn from(c: Vec<u8>) -> Self {
        Self(c)
    }
}

impl From<&[u8]> for BlobBytes {
    fn from(c: &[u8]) -> Self {
        Self(c.into())
    }
}

impl From<&mut [u8]> for BlobBytes {
    fn from(c: &mut [u8]) -> Self {
        Self(c.into())
    }
}

impl From<BlobBytes> for Vec<u8> {
    fn from(c: BlobBytes) -> Vec<u8> {
        c.0
    }
}

impl Borrow<[u8]> for BlobBytes {
    fn borrow(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for BlobBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsMut<[u8]> for BlobBytes {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

#[cfg(feature = "random")]
impl Distribution<BlobBytes> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BlobBytes {
        let len = rng.gen_range(0..1024);
        let mut val = Vec::new();
        for _ in 0..len {
            val.push(rng.r#gen());
        }
        BlobBytes(val)
    }
}

impl IntoIterator for BlobBytes {
    type IntoIter = alloc::vec::IntoIter<Self::Item>;
    type Item = u8;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

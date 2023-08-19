use fuel_types::{
    bytes,
    canonical::{
        Deserialize,
        Serialize,
    },
    Bytes32,
    Bytes64,
};

#[cfg(feature = "random")]
use rand::{
    distributions::{
        Distribution,
        Standard,
    },
    Rng,
};

use core::cmp::Ordering;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    any(feature = "alloc", feature = "std"),
    derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)
)]
pub struct StorageSlot {
    key: Bytes32,
    value: Bytes32,
}

impl StorageSlot {
    pub const SLOT_SIZE: usize = Self::SIZE_STATIC;

    pub const fn new(key: Bytes32, value: Bytes32) -> Self {
        StorageSlot { key, value }
    }

    pub const fn key(&self) -> &Bytes32 {
        &self.key
    }

    pub const fn value(&self) -> &Bytes32 {
        &self.value
    }
}

impl From<&StorageSlot> for Bytes64 {
    fn from(s: &StorageSlot) -> Self {
        let mut buf = [0u8; StorageSlot::SLOT_SIZE];

        buf[..Bytes32::LEN].copy_from_slice(s.key.as_ref());
        buf[Bytes32::LEN..].copy_from_slice(s.value.as_ref());

        buf.into()
    }
}

impl From<&Bytes64> for StorageSlot {
    fn from(b: &Bytes64) -> Self {
        // from_bytes is infallible with a fixed size array type
        let key = Bytes32::from_bytes(&b[..Bytes32::LEN]).unwrap();
        let value = Bytes32::from_bytes(&b[Bytes32::LEN..]).unwrap();
        Self::new(key, value)
    }
}

#[cfg(feature = "random")]
impl Distribution<StorageSlot> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> StorageSlot {
        StorageSlot {
            key: rng.gen(),
            value: rng.gen(),
        }
    }
}

impl bytes::SizedBytes for StorageSlot {
    fn serialized_size(&self) -> usize {
        Self::SLOT_SIZE
    }
}

impl PartialOrd for StorageSlot {
    fn partial_cmp(&self, other: &StorageSlot) -> Option<Ordering> {
        Some(self.key.cmp(&other.key))
    }
}

impl Ord for StorageSlot {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

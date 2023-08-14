use fuel_types::{
    bytes,
    mem_layout,
    Bytes32,
    Bytes64,
    MemLayout,
    MemLocType,
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
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct StorageSlot {
    key: Bytes32,
    value: Bytes32,
}

mem_layout!(StorageSlotLayout for StorageSlot
    key: Bytes32 = {Bytes32::LEN},
    value: Bytes32 = {Bytes32::LEN}
);

impl StorageSlot {
    pub const SLOT_SIZE: usize = Self::LEN;

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
        let key = bytes::restore_at(b, Self::layout(Self::LAYOUT.key)).into();
        let value = bytes::restore_at(b, Self::layout(Self::LAYOUT.value)).into();

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

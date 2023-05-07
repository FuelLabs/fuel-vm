use fuel_types::{bytes, mem_layout, Bytes32, Bytes64, MemLayout, MemLocType};

#[cfg(feature = "random")]
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

use core::cmp::Ordering;

#[cfg(feature = "std")]
use std::io;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
#[cfg_attr(feature = "rkyv", archive(check_bytes))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

#[cfg(feature = "std")]
impl io::Read for StorageSlot {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        const LEN: usize = StorageSlot::SLOT_SIZE;
        let buf: &mut [_; LEN] = buf
            .get_mut(..LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        bytes::store_at(buf, Self::layout(Self::LAYOUT.key), &self.key);
        bytes::store_at(buf, Self::layout(Self::LAYOUT.value), &self.value);

        Ok(Self::SLOT_SIZE)
    }
}

#[cfg(feature = "std")]
impl io::Write for StorageSlot {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        const LEN: usize = StorageSlot::SLOT_SIZE;
        let buf: &[_; LEN] = buf
            .get(..LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        let key = bytes::restore_at(buf, Self::layout(Self::LAYOUT.key));
        let value = bytes::restore_at(buf, Self::layout(Self::LAYOUT.value));

        self.key = key.into();
        self.value = value.into();

        Ok(Self::SLOT_SIZE)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
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

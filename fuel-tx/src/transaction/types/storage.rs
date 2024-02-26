use fuel_types::{
    canonical::{
        Deserialize,
        Serialize,
    },
    fmt_truncated_hex,
    Bytes32,
};
#[cfg(feature = "random")]
use rand::{
    distributions::{
        Distribution,
        Standard,
    },
    Rng,
};

use alloc::{
    vec,
    vec::Vec,
};
use core::cmp::Ordering;
use derivative::Derivative;
use fuel_types::canonical::{
    Error,
    Input,
    Output,
};

#[derive(Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StorageData(
    #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))] pub Vec<u8>,
);

// TODO: Remove fixed size default when adding support for dynamic storage
impl Default for StorageData {
    fn default() -> Self {
        Self(vec![0u8; 32])
    }
}

impl From<Vec<u8>> for StorageData {
    fn from(c: Vec<u8>) -> Self {
        Self(c)
    }
}

impl From<&[u8]> for StorageData {
    fn from(c: &[u8]) -> Self {
        Self(c.into())
    }
}

impl From<&mut [u8]> for StorageData {
    fn from(c: &mut [u8]) -> Self {
        Self(c.into())
    }
}

impl From<StorageData> for Vec<u8> {
    fn from(c: StorageData) -> Vec<u8> {
        c.0
    }
}

impl AsRef<[u8]> for StorageData {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsMut<[u8]> for StorageData {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

#[cfg(feature = "random")]
impl Distribution<StorageData> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> StorageData {
        StorageData(rng.gen::<Bytes32>().to_vec())
    }
}

impl IntoIterator for StorageData {
    type IntoIter = alloc::vec::IntoIter<Self::Item>;
    type Item = u8;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// TODO: Remove manual serialization when implementing dynamic storage
// StorageData uses Vec<u8> to represent storage data. We manually implement
// Serialize and Deserialize in order to preserve the 32-byte serialized format
// of the StorageData, and by extension, the Create Transaction. When it is
// possible to write dynamically sized storage data via new opcodes, and
// dynamically sized slots are supported, remove these implementations, allowing
// native support for dynamically sized serialization and deserialization.
impl Serialize for StorageData {
    fn size_static(&self) -> usize {
        Bytes32::LEN
    }

    fn size_dynamic(&self) -> usize {
        0
    }

    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        let mut bytes = Bytes32::default();
        bytes[0..Bytes32::LEN].copy_from_slice(&self.0[0..Bytes32::LEN]);
        bytes.encode_static(buffer)
    }

    fn encode_dynamic<O: Output + ?Sized>(&self, _buffer: &mut O) -> Result<(), Error> {
        Ok(())
    }
}

// TODO: Remove manual deserialization when implementing dynamic storage
impl Deserialize for StorageData {
    fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
        let bytes = Bytes32::decode(buffer)?;
        let data = Self::from(bytes.as_ref());
        Ok(data)
    }

    fn decode_dynamic<I: Input + ?Sized>(
        &mut self,
        _buffer: &mut I,
    ) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
#[derive(Deserialize, Serialize)]
pub struct StorageSlot {
    key: Bytes32,
    value: StorageData,
}

impl StorageSlot {
    pub const fn new(key: Bytes32, value: StorageData) -> Self {
        StorageSlot { key, value }
    }

    pub const fn key(&self) -> &Bytes32 {
        &self.key
    }

    pub const fn value(&self) -> &StorageData {
        &self.value
    }

    pub fn size(&self) -> usize {
        Serialize::size(self)
    }
}

impl From<&(Bytes32, StorageData)> for StorageSlot {
    fn from((key, value): &(Bytes32, StorageData)) -> Self {
        Self::new(*key, value.clone())
    }
}

#[cfg(feature = "random")]
impl Distribution<StorageSlot> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> StorageSlot {
        StorageSlot {
            key: rng.gen(),
            value: rng.gen::<StorageData>(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{
        RngCore,
        SeedableRng,
    };
    use std::{
        fs::File,
        path::PathBuf,
    };

    const FILE_PATH: &str = "storage-slots.json";

    #[test]
    fn test_storage_slot_serde_serialization_creates_storage_slot() {
        // Given
        let rng = &mut rand::rngs::StdRng::seed_from_u64(8586);
        let key: Bytes32 = rng.gen();
        let value = rng.gen::<StorageData>();
        let slot = StorageSlot::new(key, value);
        let slots = vec![slot.clone()];

        // When
        let slot_str = serde_json::to_string(&slots).expect("to string");
        let storage_slots: Vec<StorageSlot> =
            serde_json::from_str(&slot_str).expect("read from string");

        // Then
        assert_eq!(storage_slots.len(), 1);
    }

    fn test_storage_slot_serde_serialization_from_file_creates_storage_slot() {
        // Given
        let path = std::env::temp_dir().join(PathBuf::from(FILE_PATH));
        let storage_slots_file = File::create(&path).expect("create file");
        serde_json::to_writer(&storage_slots_file, &slots).expect("write file");
        let storage_slots_file = File::open(&path).expect("open file");

        // When
        let storage_slots: Vec<StorageSlot> =
            serde_json::from_reader(storage_slots_file).expect("read file");

        // Then
        assert_eq!(storage_slots.len(), 1);
    }

    #[test]
    fn test_storage_slot_canonical_serialization_from_bytes_creates_storage_slot() {
        // Given
        let rng = &mut rand::rngs::StdRng::seed_from_u64(8586);
        let key: Bytes32 = rng.gen();
        let mut value = StorageData::from(vec![0u8; 32]);
        rng.fill_bytes(value.as_mut());
        let slot = StorageSlot::new(key, value.clone());

        // When
        let slot_bytes = slot.to_bytes();
        let (slot_key, slot_value) = slot_bytes.split_at(32);
        let recreated_slot =
            StorageSlot::from_bytes(&slot_bytes).expect("read from bytes");

        // Then
        assert_eq!(slot_key, key.as_ref());
        assert_eq!(slot_value, value.as_ref());
        assert_eq!(recreated_slot, slot);
    }

    #[test]
    fn test_storage_slot_size_returns_expected_size() {
        // Given
        let rng = &mut rand::rngs::StdRng::seed_from_u64(8586);
        let key: Bytes32 = rng.gen();
        let mut value = StorageData::from(vec![0u8; 32]);
        rng.fill_bytes(value.as_mut());

        // When
        let slot = StorageSlot::new(key, value);
        let size = slot.size();

        // Then
        let expected_size = 64; // 32-byte Key + 32-byte Data
        assert_eq!(size, expected_size);
    }
}

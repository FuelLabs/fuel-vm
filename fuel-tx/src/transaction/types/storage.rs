use alloc::vec::Vec;
use fuel_types::{
    canonical::{
        Deserialize,
        Serialize,
    },
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

use core::cmp::Ordering;

pub type StorageData = Vec<u8>;

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
            value: rng.gen::<Bytes32>().to_vec(),
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
    fn test_storage_slot_serialization() {
        let rng = &mut rand::rngs::StdRng::seed_from_u64(8586);
        let key: Bytes32 = rng.gen();
        let value = rng.gen::<Bytes32>().to_vec();

        let slot = StorageSlot::new(key, value);
        let slots = vec![slot.clone()];

        // `from_str` works
        let slot_str = serde_json::to_string(&slots).expect("to string");
        let storage_slots: Vec<StorageSlot> =
            serde_json::from_str(&slot_str).expect("read from string");
        assert_eq!(storage_slots.len(), 1);

        let path = std::env::temp_dir().join(PathBuf::from(FILE_PATH));

        // writes to file works
        let storage_slots_file = File::create(&path).expect("create file");
        serde_json::to_writer(&storage_slots_file, &slots).expect("write file");

        // `from_reader` works
        let storage_slots_file = File::open(&path).expect("open file");
        let storage_slots: Vec<StorageSlot> =
            serde_json::from_reader(storage_slots_file).expect("read file");
        assert_eq!(storage_slots.len(), 1);
    }

    #[test]
    fn test_storage_slot_canonical_serialization() {
        let rng = &mut rand::rngs::StdRng::seed_from_u64(8586);
        let key: Bytes32 = rng.gen();
        let mut value = [0u8; 128];
        rng.fill_bytes(&mut value);

        let slot = StorageSlot::new(key, value.to_vec());

        let slot_bytes = slot.to_bytes();

        let (slot_key, slot_data) = slot_bytes.split_at(32);

        assert_eq!(slot_key, key.as_ref());

        let slot_data_num_bytes =
            u64::from_bytes(&slot_data[..8]).expect("read from bytes");
        assert_eq!(slot_data_num_bytes, 128);

        // `from_bytes` works
        let recreated_slot =
            StorageSlot::from_bytes(&slot_bytes).expect("read from bytes");
        assert_eq!(recreated_slot, slot);
    }

    #[test]
    fn test_storage_slot_size() {
        let rng = &mut rand::rngs::StdRng::seed_from_u64(8586);
        let key: Bytes32 = rng.gen();
        let mut value = [0u8; 128];
        rng.fill_bytes(&mut value);

        let slot = StorageSlot::new(key, value.to_vec());
        let size = slot.size();
        let expected_size = 32 + 8 + 128; // Key + u64 (data size) + Data
        assert_eq!(size, expected_size);
    }
}

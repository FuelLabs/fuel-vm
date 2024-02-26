use fuel_types::{
    canonical::{
        Deserialize,
        Serialize,
    },
    fmt_truncated_hex,
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

use alloc::{
    vec,
    vec::Vec,
};
use core::cmp::Ordering;
use derivative::Derivative;

#[derive(Derivative, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ContractsStateData(
    #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))] pub Vec<u8>,
);

// TODO: Remove fixed size default when adding support for dynamic storage
impl Default for ContractsStateData {
    fn default() -> Self {
        Self(vec![0u8; 32])
    }
}

impl From<Vec<u8>> for ContractsStateData {
    fn from(c: Vec<u8>) -> Self {
        Self(c)
    }
}

impl From<&[u8]> for ContractsStateData {
    fn from(c: &[u8]) -> Self {
        Self(c.into())
    }
}

impl From<&mut [u8]> for ContractsStateData {
    fn from(c: &mut [u8]) -> Self {
        Self(c.into())
    }
}

impl From<ContractsStateData> for Vec<u8> {
    fn from(c: ContractsStateData) -> Vec<u8> {
        c.0
    }
}

impl AsRef<[u8]> for ContractsStateData {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsMut<[u8]> for ContractsStateData {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

#[cfg(feature = "random")]
impl Distribution<ContractsStateData> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ContractsStateData {
        ContractsStateData(rng.gen::<Bytes32>().to_vec())
    }
}

impl IntoIterator for ContractsStateData {
    type IntoIter = alloc::vec::IntoIter<Self::Item>;
    type Item = u8;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
#[derive(Deserialize, Serialize)]
pub struct StorageSlot {
    key: Bytes32,
    value: Bytes32,
}

impl StorageSlot {
    pub const SLOT_SIZE: usize = Bytes32::LEN + Bytes32::LEN;

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

// impl From<&(Bytes32, ContractsStateData)> for StorageSlot {
//     fn from((key, value): &(Bytes32, ContractsStateData)) -> Self {
//         Self::new(*key, value.clone())
//     }
// }

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
        let key = <Bytes32 as Deserialize>::from_bytes(&b[..Bytes32::LEN])
            .expect("Infallible deserialization");
        let value = <Bytes32 as Deserialize>::from_bytes(&b[Bytes32::LEN..])
            .expect("Infallible deserialization");
        Self::new(key, value)
    }
}

#[cfg(feature = "random")]
impl Distribution<StorageSlot> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> StorageSlot {
        StorageSlot {
            key: rng.gen(),
            value: rng.gen(),
            // value: rng.gen::<ContractsStateData>(),
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
    use rand::SeedableRng;
    use std::{
        fs::File,
        path::PathBuf,
    };

    const FILE_PATH: &str = "storage-slots.json";

    #[test]
    fn test_storage_slot_serialization() {
        let rng = &mut rand::rngs::StdRng::seed_from_u64(8586);
        let key: Bytes32 = rng.gen();
        let value: Bytes32 = rng.gen();

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
}

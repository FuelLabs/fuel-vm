use fuel_types::{
    canonical::{Deserialize, Serialize},
    Bytes32, Bytes64,
};

#[cfg(feature = "random")]
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

use core::cmp::Ordering;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

#[test]
#[cfg(feature = "serde")]
fn test_storage_slot_serialization() {
    use std::fs::File;
    use std::path::PathBuf;

    use rand::SeedableRng;

    let rng = &mut rand::rngs::StdRng::seed_from_u64(8586);

    let key: Bytes32 = rng.gen();
    let value: Bytes32 = rng.gen();

    let slot = StorageSlot::new(key, value);
    let slots = vec![slot.clone()];

    // writes to file
    let storage_slots_file =
        File::create(PathBuf::from("storage-slots.json")).expect("create file");
    let res = serde_json::to_writer(&storage_slots_file, &slots).expect("write file");

    // from string works
    let slot_str = serde_json::to_string(&slots).expect("to string");
    let storage_slots: Vec<StorageSlot> =
        serde_json::from_str(&slot_str).expect("read from string");
    assert_eq!(storage_slots.len(), 1);

    // this fails
    let storage_slots_file =
        std::fs::File::open(PathBuf::from("storage-slots.json")).expect("open file");
    let storage_slots: Vec<StorageSlot> =
        serde_json::from_reader(storage_slots_file).expect("read file");
    assert_eq!(storage_slots.len(), 1);
}

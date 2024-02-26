use crate::ContractsStateData;
use fuel_types::{
    canonical::{
        Deserialize,
        Serialize,
    },
    Bytes32,
};

use super::*;

#[test]
fn test_create_serialization() {
    let create = Create {
        storage_slots: vec![
            StorageSlot::new(
                Bytes32::from([1u8; 32]),
                ContractsStateData::from([2u8; 32].as_ref()),
            ),
            StorageSlot::new(
                Bytes32::from([3u8; 32]),
                ContractsStateData::from([4u8; 32].as_ref()),
            ),
        ],

        ..Default::default()
    };
    let bytes = create.to_bytes();
    let create2 = Create::from_bytes(&bytes).unwrap();
    assert_eq!(create, create2);
}

use fuel_types::{
    Bytes32,
    canonical::{
        Deserialize,
        Serialize,
    },
};

use super::*;

#[test]
fn test_create_serialization() {
    let create = Create {
        body: CreateBody {
            storage_slots: vec![
                StorageSlot::new(Bytes32::from([1u8; 32]), Bytes32::from([2u8; 32])),
                StorageSlot::new(Bytes32::from([3u8; 32]), Bytes32::from([4u8; 32])),
            ],
            ..Default::default()
        },
        ..Default::default()
    };
    let bytes = create.to_bytes();
    let create2 = Create::from_bytes(&bytes).unwrap();
    assert_eq!(create, create2);
}

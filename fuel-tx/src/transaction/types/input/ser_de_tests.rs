use super::*;
use fuel_types::canonical::{
    Deserialize,
    Serialize,
};

#[test]
fn test_input_serialization() {
    const DATA_SIZE: usize = 16;
    let input = Input::message_data_predicate(
        Address::from([2u8; 32]),
        Address::from([3u8; 32]),
        5,
        Nonce::from([6u8; 32]),
        100_000,
        vec![7u8; DATA_SIZE],
        vec![8u8; DATA_SIZE],
        vec![9u8; DATA_SIZE],
    );
    let bytes = input.to_bytes();
    let mut r = 0..8;
    assert_eq!(bytes[r.clone()], 2u64.to_be_bytes()); // discriminant (InputRepr)
    r.start = r.end;
    r.end += 32;
    assert_eq!(bytes[r.clone()], [2u8; 32]); // sender
    r.start = r.end;
    r.end += 32;
    assert_eq!(bytes[r.clone()], [3u8; 32]); // recipient
    r.start = r.end;
    r.end += 8;
    assert_eq!(bytes[r.clone()], 5u64.to_be_bytes()); // amount
    r.start = r.end;
    r.end += 32;
    assert_eq!(bytes[r.clone()], [6u8; 32]); // nonce
    r.start = r.end;
    r.end += 8;
    assert_eq!(bytes[r.clone()], 0u64.to_be_bytes()); // witness_index
    r.start = r.end;
    r.end += 8;
    assert_eq!(bytes[r.clone()], 100_000u64.to_be_bytes()); // predicate_gas_used
    r.start = r.end;
    r.end += 8;
    assert_eq!(bytes[r.clone()], (DATA_SIZE as u64).to_be_bytes()); // data_len
    r.start = r.end;
    r.end += 8;
    assert_eq!(bytes[r.clone()], (DATA_SIZE as u64).to_be_bytes()); // predicate_len
    r.start = r.end;
    r.end += 8;
    assert_eq!(bytes[r.clone()], (DATA_SIZE as u64).to_be_bytes()); // predicate_data_len
    r.start = r.end;
    r.end += DATA_SIZE;
    assert_eq!(bytes[r.clone()], [7u8; DATA_SIZE]); // data
    r.start = r.end;
    r.end += DATA_SIZE;
    assert_eq!(bytes[r.clone()], [8u8; DATA_SIZE]); // predicate
    r.start = r.end;
    r.end += DATA_SIZE;
    assert_eq!(bytes[r.clone()], [9u8; DATA_SIZE]); // predicate_data
    assert_eq!(r.end, bytes.len());
    let input2 = Input::from_bytes(&bytes).unwrap();
    assert_eq!(input, input2);
}

#[cfg(feature = "u32-tx-pointer")]
#[test]
fn tx_with_coin_input() {
    use crate::TransactionBuilder;

    // Given
    let tx_u32 = TransactionBuilder::script(vec![], vec![])
        .tip(1)
        .maturity(123.into())
        .expiration(456.into())
        .max_fee_limit(1000000)
        .add_input(Input::CoinSigned(CoinSigned {
            utxo_id: UtxoId::new([1u8; 32].into(), 2),
            owner: [2u8; 32].into(),
            amount: 11,
            asset_id: [5u8; 32].into(),
            tx_pointer: TxPointer::new(46.into(), u32::MAX),
            witness_index: 4,
            predicate_gas_used: Empty::new(),
            predicate: Empty::new(),
            predicate_data: Empty::new(),
        }))
        .finalize_as_transaction();

    let tx_u16 = TransactionBuilder::script(vec![], vec![])
        .tip(1)
        .maturity(123.into())
        .expiration(456.into())
        .max_fee_limit(1000000)
        .add_input(Input::CoinSigned(CoinSigned {
            utxo_id: UtxoId::new([1u8; 32].into(), 2),
            owner: [2u8; 32].into(),
            amount: 11,
            asset_id: [5u8; 32].into(),
            tx_pointer: TxPointer::new(46.into(), u16::MAX.into()),
            witness_index: 4,
            predicate_gas_used: Empty::new(),
            predicate: Empty::new(),
            predicate_data: Empty::new(),
        }))
        .finalize_as_transaction();

    // When
    let bytes_u32 = postcard::to_allocvec(&tx_u32).unwrap();
    let bytes_u16 = postcard::to_allocvec(&tx_u16).unwrap();

    // Then
    assert_eq!(bytes_u32.len(), bytes_u16.len() + 2);
}

#[cfg(feature = "u32-tx-pointer")]
#[test]
fn decode_tx_pointer_from_u16() {
    use fuel_types::BlockHeight;

    // Given (arbitrary big number)
    let old_version_pointer: (BlockHeight, u16) =
        (BlockHeight::from(123), u16::MAX - 3489);
    let bytes = postcard::to_allocvec(&old_version_pointer).unwrap();

    // When
    let new_version_pointer: TxPointer = postcard::from_bytes(&bytes).unwrap();

    // Then
    assert_eq!(
        new_version_pointer,
        TxPointer::new(123.into(), (u16::MAX - 3489).into())
    );
}

use super::*;
use crate::input::sizes::{
    MessageSizes,
    MessageSizesLayout,
};
use fuel_types::{
    canonical::{
        Deserialize,
        Serialize,
    },
    MemLayout,
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
    const S: MessageSizesLayout = MessageSizes::LAYOUT;
    assert_eq!(
        input.serialized_size(),
        WORD_SIZE
            + S.sender.size()
            + S.recipient.size()
            + S.amount.size()
            + S.nonce.size()
            + S.witness_index.size()
            + S.predicate_gas_used.size()
            + S.data_len.size()
            + S.predicate_len.size()
            + S.predicate_data_len.size()
            + DATA_SIZE
            + DATA_SIZE
            + DATA_SIZE
    );
    assert_eq!(
        input.serialized_size(),
        WORD_SIZE + MessageSizesLayout::LEN + DATA_SIZE + DATA_SIZE + DATA_SIZE
    );
    println!("====");
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
    println!("From bytes: {:?}", bytes);
    let input2 = Input::from_bytes(&bytes).unwrap();
    assert_eq!(input, input2);
}

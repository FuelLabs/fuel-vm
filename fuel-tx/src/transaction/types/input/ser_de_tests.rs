use super::*;
use crate::input::sizes::{MessageSizes, MessageSizesLayout};
use fuel_types::bytes::Deserializable;
use fuel_types::bytes::SerializableVec;
use fuel_types::MemLayout;

#[test]
fn test_input_serialization() {
    const DATA_SIZE: usize = 16;
    let mut input = Input::message_predicate(
        MessageId::from([1u8; 32]),
        Address::from([2u8; 32]),
        Address::from([3u8; 32]),
        5,
        6,
        vec![7u8; DATA_SIZE],
        vec![8u8; DATA_SIZE],
        vec![9u8; DATA_SIZE],
    );
    const S: MessageSizesLayout = MessageSizes::LAYOUT;
    assert_eq!(
        input.serialized_size(),
        WORD_SIZE
            + S.message_id.size()
            + S.sender.size()
            + S.recipient.size()
            + S.amount.size()
            + S.nonce.size()
            + S.witness_index.size()
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
    let bytes = input.to_bytes();
    let mut r = 0..8;
    assert_eq!(bytes[r.clone()], 2u64.to_be_bytes());
    r.start = r.end;
    r.end += 32;
    assert_eq!(bytes[r.clone()], [1u8; 32]);
    r.start = r.end;
    r.end += 32;
    assert_eq!(bytes[r.clone()], [2u8; 32]);
    r.start = r.end;
    r.end += 32;
    assert_eq!(bytes[r.clone()], [3u8; 32]);
    r.start = r.end;
    r.end += 8;
    assert_eq!(bytes[r.clone()], 5u64.to_be_bytes());
    r.start = r.end;
    r.end += 8;
    assert_eq!(bytes[r.clone()], 6u64.to_be_bytes());
    r.start = r.end;
    r.end += 8;
    assert_eq!(bytes[r.clone()], 0u64.to_be_bytes());
    r.start = r.end;
    r.end += 8;
    assert_eq!(bytes[r.clone()], 16u64.to_be_bytes());
    r.start = r.end;
    r.end += 8;
    assert_eq!(bytes[r.clone()], 16u64.to_be_bytes());
    r.start = r.end;
    r.end += 8;
    assert_eq!(bytes[r.clone()], 16u64.to_be_bytes());
    r.start = r.end;
    r.end += DATA_SIZE;
    assert_eq!(bytes[r.clone()], [7u8; DATA_SIZE]);
    r.start = r.end;
    r.end += DATA_SIZE;
    assert_eq!(bytes[r.clone()], [8u8; DATA_SIZE]);
    r.start = r.end;
    r.end += DATA_SIZE;
    assert_eq!(bytes[r], [9u8; DATA_SIZE]);
    let input2 = Input::from_bytes(&bytes).unwrap();
    assert_eq!(input, input2);
}

use fuel_types::bytes::{
    self,
    WORD_SIZE,
};

#[test]
#[allow(clippy::erasing_op)]
#[allow(clippy::identity_op)]
fn padded_len_to_fit_word_len() {
    assert_eq!(WORD_SIZE * 0, bytes::padded_len(&[]));
    assert_eq!(WORD_SIZE * 1, bytes::padded_len(&[0]));
    assert_eq!(WORD_SIZE * 1, bytes::padded_len(&[0; WORD_SIZE]));
    assert_eq!(WORD_SIZE * 2, bytes::padded_len(&[0; WORD_SIZE + 1]));
    assert_eq!(WORD_SIZE * 2, bytes::padded_len(&[0; WORD_SIZE * 2]));
}

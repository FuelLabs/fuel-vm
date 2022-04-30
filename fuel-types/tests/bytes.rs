use fuel_types::bytes::{self, WORD_SIZE};
use fuel_types::Word;

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

#[test]
fn store_restore_number_unchecked_works() {
    fn store_restore<T>(n: T, x: usize, f: unsafe fn(&[u8]) -> (T, &[u8]))
    where
        T: core::fmt::Debug + Copy + Eq,
        Word: From<T>,
    {
        let mut buffer = [0u8; 255];

        assert_eq!(
            0,
            bytes::store_number_unchecked(&mut buffer[..WORD_SIZE], n).len()
        );
        assert_eq!(n, unsafe { f(&buffer).0 });
        assert_eq!(0, unsafe { f(&buffer[..WORD_SIZE]).1.len() });

        assert_eq!(
            x,
            bytes::store_number_unchecked(&mut buffer[..WORD_SIZE + x], n).len()
        );
        assert_eq!(n, unsafe { f(&buffer).0 });
        assert_eq!(x, unsafe { f(&buffer[..WORD_SIZE + x]).1.len() });
    }

    store_restore::<Word>(65, 5, bytes::restore_number_unchecked);
    store_restore::<Word>(65, 5, bytes::restore_word_unchecked);
    store_restore::<u8>(65, 5, bytes::restore_u8_unchecked);
    store_restore::<u16>(65, 5, bytes::restore_u16_unchecked);
    store_restore::<u32>(65, 5, bytes::restore_u32_unchecked);
}

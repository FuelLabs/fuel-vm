use crate::Word;

/// Size of a word, in bytes
pub const WORD_SIZE: usize = core::mem::size_of::<Word>();

/// Return the word-padded length of the buffer.
/// Returns None if the length is too large to be represented as usize.
pub const fn padded_len(bytes: &[u8]) -> Option<usize> {
    padded_len_usize(bytes.len())
}

/// Return the word-padded length of an arbitrary length.
/// Returns None if the length is too large to be represented as usize.
#[allow(clippy::arithmetic_side_effects)] // Safety: (a % b) < b
pub const fn padded_len_usize(len: usize) -> Option<usize> {
    let modulo = len % WORD_SIZE;
    if modulo == 0 {
        Some(len)
    } else {
        let padding = WORD_SIZE - modulo;
        len.checked_add(padding)
    }
}

/// Return the word-padded length of an arbitrary length.
/// Returns None if the length is too large to be represented as `Word`.
#[allow(clippy::arithmetic_side_effects)] // Safety: (a % b) < b
pub const fn padded_len_word(len: Word) -> Option<Word> {
    let modulo = len % WORD_SIZE as Word;
    if modulo == 0 {
        Some(len)
    } else {
        let padding = WORD_SIZE as Word - modulo;
        len.checked_add(padding)
    }
}

#[cfg(feature = "unsafe")]
#[allow(unsafe_code)]
/// Add a conversion from arbitrary slices into arrays
///
/// # Safety
///
/// This function will not panic if the length of the slice is smaller than `N`. Instead,
/// it will cause undefined behavior and read random disowned bytes.
pub unsafe fn from_slice_unchecked<const N: usize>(buf: &[u8]) -> [u8; N] {
    unsafe {
        let ptr = buf.as_ptr() as *const [u8; N];

        // Static assertions are not applicable to runtime length check (e.g. slices).
        // This is safe if the size of `bytes` is consistent to `N`
        *ptr
    }
}

#[test]
#[allow(clippy::erasing_op)]
#[allow(clippy::identity_op)]
fn padded_len_returns_multiple_of_word_len() {
    assert_eq!(Some(WORD_SIZE * 0), padded_len(&[]));
    assert_eq!(Some(WORD_SIZE * 1), padded_len(&[0]));
    assert_eq!(Some(WORD_SIZE * 1), padded_len(&[0; WORD_SIZE]));
    assert_eq!(Some(WORD_SIZE * 2), padded_len(&[0; WORD_SIZE + 1]));
    assert_eq!(Some(WORD_SIZE * 2), padded_len(&[0; WORD_SIZE * 2]));
}

#[test]
fn padded_len_usize_returns_multiple_of_word_len() {
    assert_eq!(padded_len_usize(0), Some(0));
    assert_eq!(padded_len_usize(1), Some(8));
    assert_eq!(padded_len_usize(2), Some(8));
    assert_eq!(padded_len_usize(7), Some(8));
    assert_eq!(padded_len_usize(8), Some(8));
    assert_eq!(padded_len_usize(9), Some(16));
}

#[test]
fn padded_len_usize_handles_overflow() {
    for i in 0..7 {
        assert_eq!(padded_len_usize(usize::MAX - i), None);
    }
    assert_eq!(padded_len_usize(usize::MAX - 7), Some(usize::MAX - 7));
}

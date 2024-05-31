use crate::Word;

/// Size of a word, in bytes
pub const WORD_SIZE: usize = core::mem::size_of::<Word>();

/// Return the word-padded length of the buffer
pub const fn padded_len(bytes: &[u8]) -> usize {
    padded_len_usize(bytes.len())
}

/// Return the word-padded length of an arbitrary length
pub const fn padded_len_word(len: Word) -> Word {
    let pad = len % (WORD_SIZE as Word);

    // `pad != 0` is checked because we shouldn't pad in case the length is already
    // well-formed.
    //
    // Example being `w := WORD_SIZE` and `x := 2 · w`
    //
    // 1) With the check (correct result)
    // f(x) -> x + (x % w != 0) · (w - x % w)
    // f(x) -> x + 0 · w
    // f(x) -> x
    //
    // 2) Without the check (incorrect result)
    // f(x) -> x + w - x % w
    // f(x) -> x + w
    len + (pad != 0) as Word * ((WORD_SIZE as Word) - pad)
}

/// Return the word-padded length of an arbitrary length
pub const fn padded_len_usize(len: usize) -> usize {
    let pad = len % WORD_SIZE;

    // `pad != 0` is checked because we shouldn't pad in case the length is already
    // well-formed.
    //
    // Example being `w := WORD_SIZE` and `x := 2 · w`
    //
    // 1) With the check (correct result)
    // f(x) -> x + (x % w != 0) · (w - x % w)
    // f(x) -> x + 0 · w
    // f(x) -> x
    //
    // 2) Without the check (incorrect result)
    // f(x) -> x + w - x % w
    // f(x) -> x + w
    len + (pad != 0) as usize * (WORD_SIZE - pad)
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
    let ptr = buf.as_ptr() as *const [u8; N];

    // Static assertions are not applicable to runtime length check (e.g. slices).
    // This is safe if the size of `bytes` is consistent to `N`
    *ptr
}

#[test]
#[allow(clippy::erasing_op)]
#[allow(clippy::identity_op)]
fn padded_len_to_fit_word_len() {
    assert_eq!(WORD_SIZE * 0, padded_len(&[]));
    assert_eq!(WORD_SIZE * 1, padded_len(&[0]));
    assert_eq!(WORD_SIZE * 1, padded_len(&[0; WORD_SIZE]));
    assert_eq!(WORD_SIZE * 2, padded_len(&[0; WORD_SIZE + 1]));
    assert_eq!(WORD_SIZE * 2, padded_len(&[0; WORD_SIZE * 2]));
}

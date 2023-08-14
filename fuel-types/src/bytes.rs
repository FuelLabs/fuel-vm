use crate::Word;

pub use const_layout::*;

mod const_layout;

/// Define the amount of bytes for a serialization implementation.
pub trait SizedBytes {
    /// Return the expected serialized size for an instance of the type.
    fn serialized_size(&self) -> usize;
}

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

/// Store a number into this buffer.
pub fn store_number<T>(buf: &mut [u8; WORD_SIZE], number: T)
where
    T: Into<Word>,
{
    buf.copy_from_slice(&number.into().to_be_bytes());
}

/// Read a number from a buffer.
pub fn restore_number<T>(buf: [u8; WORD_SIZE]) -> T
where
    T: From<Word>,
{
    Word::from_be_bytes(buf).into()
}

/// Read a word from a buffer.
pub fn restore_word(buf: [u8; WORD_SIZE]) -> Word {
    Word::from_be_bytes(buf)
}

/// Read a word-padded u8 from a buffer.
pub fn restore_u8(buf: [u8; WORD_SIZE]) -> u8 {
    Word::from_be_bytes(buf) as u8
}

/// Read the a word-padded u16 from a buffer.
pub fn restore_u16(buf: [u8; WORD_SIZE]) -> u16 {
    Word::from_be_bytes(buf) as u16
}

/// Read the a word-padded u32 from a buffer.
pub fn restore_u32(buf: [u8; WORD_SIZE]) -> u32 {
    Word::from_be_bytes(buf) as u32
}

/// Read the a word-padded usize from a buffer.
pub fn restore_usize(buf: [u8; WORD_SIZE]) -> usize {
    Word::from_be_bytes(buf) as usize
}

/// Read an array of `N` bytes from `buf`.
///
/// # Panics
///
/// This function will panic if the length of `buf` is smaller than `N`
pub fn restore_array_from_slice<const N: usize>(buf: &[u8]) -> [u8; N] {
    buf.try_into().expect("buf must be at least N bytes long")
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

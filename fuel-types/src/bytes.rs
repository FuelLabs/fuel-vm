use crate::Word;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "alloc")]
/// A new type around `Vec<u8>` with useful utilities and optimizations.
#[derive(educe::Educe, Default, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[educe(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde_with::serde_as)]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct Bytes(
    #[educe(Debug(method(crate::fmt::fmt_truncated_hex::<16>)))]
    #[cfg_attr(feature = "serde", serde_as(as = "Bytes"))]
    Vec<u8>,
);

#[cfg(feature = "alloc")]
impl Bytes {
    /// Creates a new `Bytes` from a `Vec<u8>`.
    pub const fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Consumes the `Bytes`, returning the underlying `Vec<u8>`.
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

#[cfg(feature = "alloc")]
impl core::ops::Deref for Bytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "alloc")]
impl core::ops::DerefMut for Bytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "alloc")]
impl From<Vec<u8>> for Bytes {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

#[cfg(feature = "alloc")]
impl From<Bytes> for Vec<u8> {
    fn from(value: Bytes) -> Self {
        value.0
    }
}

#[cfg(feature = "alloc")]
impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(feature = "alloc")]
impl AsMut<[u8]> for Bytes {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

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

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use crate::bytes::{
        Bytes,
        WORD_SIZE,
        padded_len,
        padded_len_usize,
    };

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

    #[test]
    fn bytes__postcard__serialization_correct() {
        // Given
        let original_bytes = vec![1u8, 2u8, 3u8, 4u8, 5u8];
        let bytes = Bytes::new(original_bytes.clone());

        // When
        let serialized_bytes = postcard::to_allocvec(&bytes).unwrap();
        let serialized_original_bytes = postcard::to_allocvec(&original_bytes).unwrap();

        // Then
        assert_eq!(serialized_bytes, serialized_original_bytes);
    }

    #[test]
    fn bytes__postcard__deserialization_correct() {
        // Given
        let original_bytes = vec![1u8, 2u8, 3u8, 4u8, 5u8];
        let serialized_original_bytes = postcard::to_allocvec(&original_bytes).unwrap();

        // When
        let deserialized_bytes: Bytes =
            postcard::from_bytes(&serialized_original_bytes).unwrap();
        let expected_bytes = Bytes::new(original_bytes);

        // Then
        assert_eq!(deserialized_bytes, expected_bytes);
    }

    #[test]
    fn bytes__bincode__serialization_correct() {
        // Given
        let original_bytes = vec![1u8, 2u8, 3u8, 4u8, 5u8];
        let bytes = Bytes::new(original_bytes.clone());

        // When
        let serialized_bytes = bincode::serialize(&bytes).unwrap();
        let serialized_original_bytes = bincode::serialize(&original_bytes).unwrap();

        // Then
        assert_eq!(serialized_bytes, serialized_original_bytes);
    }

    #[test]
    fn bytes__bincode__deserialization_correct() {
        // Given
        let original_bytes = vec![1u8, 2u8, 3u8, 4u8, 5u8];
        let serialized_original_bytes = bincode::serialize(&original_bytes).unwrap();

        // When
        let deserialized_bytes: Bytes =
            bincode::deserialize(&serialized_original_bytes).unwrap();
        let expected_bytes = Bytes::new(original_bytes);

        // Then
        assert_eq!(deserialized_bytes, expected_bytes);
    }

    #[test]
    fn bytes__json__serialization_correct() {
        // Given
        let original_bytes = vec![1u8, 2u8, 3u8, 4u8, 5u8];
        let bytes = Bytes::new(original_bytes.clone());

        // When
        let serialized_bytes = serde_json::to_string(&bytes).unwrap();
        let serialized_original_bytes = serde_json::to_string(&original_bytes).unwrap();

        // Then
        assert_eq!(serialized_bytes, serialized_original_bytes);
    }

    #[test]
    fn bytes__json__deserialization_correct() {
        // Given
        let original_bytes = vec![1u8, 2u8, 3u8, 4u8, 5u8];
        let serialized_original_bytes = serde_json::to_string(&original_bytes).unwrap();

        // When
        let deserialized_bytes: Bytes =
            serde_json::from_str(&serialized_original_bytes).unwrap();
        let expected_bytes = Bytes::new(original_bytes);

        // Then
        assert_eq!(deserialized_bytes, expected_bytes);
    }
}

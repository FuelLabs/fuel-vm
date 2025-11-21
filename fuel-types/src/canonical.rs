//! Canonical serialization and deserialization of Fuel types.
//!
//! This module provides the `Serialize` and `Deserialize` traits, which
//! allow for automatic serialization and deserialization of Fuel types.

#![allow(unsafe_code)]

#[cfg(feature = "alloc")]
use alloc::{
    vec,
    vec::Vec,
};
use core::fmt;

use core::mem::MaybeUninit;
pub use fuel_derive::{
    Deserialize,
    Serialize,
};

/// Error when serializing or deserializing.
#[derive(Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// The buffer is to short for writing or reading.
    BufferIsTooShort,
    /// Got unknown enum's discriminant.
    UnknownDiscriminant,
    /// Struct prefix (set with `#[canonical(prefix = ...)]`) was invalid.
    InvalidPrefix,
    /// Allocation too large to be correct.
    AllocationLimit,
    /// Unknown error.
    Unknown(&'static str),
}

impl Error {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Error::BufferIsTooShort => "buffer is too short",
            Error::UnknownDiscriminant => "unknown discriminant",
            Error::InvalidPrefix => {
                "prefix set with #[canonical(prefix = ...)] was invalid"
            }
            Error::AllocationLimit => "allocation too large",
            Error::Unknown(str) => str,
        }
    }
}

impl fmt::Display for Error {
    /// Shows a human-readable description of the `Error`.
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(self.as_str())
    }
}

/// Allows writing of data.
pub trait Output {
    /// Write bytes to the output buffer.
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error>;

    /// Write a single byte to the output buffer.
    fn push_byte(&mut self, byte: u8) -> Result<(), Error> {
        self.write(&[byte])
    }
}

/// Allows serialize the type into the `Output`.
/// https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/tx_format.md#transaction
pub trait Serialize {
    /// !INTERNAL USAGE ONLY!
    /// Array of bytes that are now aligned by themselves.
    #[doc(hidden)]
    const UNALIGNED_BYTES: bool = false;

    /// Size of the static part of the serialized object, in bytes.
    /// Saturates to usize::MAX on overflow.
    fn size_static(&self) -> usize;

    /// Size of the dynamic part, in bytes.
    /// Saturates to usize::MAX on overflow.
    fn size_dynamic(&self) -> usize;

    /// Total size of the serialized object, in bytes.
    /// Saturates to usize::MAX on overflow.
    fn size(&self) -> usize {
        self.size_static().saturating_add(self.size_dynamic())
    }

    /// Encodes `Self` into the `buffer`.
    ///
    /// It is better to not implement this function directly, instead implement
    /// `encode_static` and `encode_dynamic`.
    fn encode<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        self.encode_static(buffer)?;
        self.encode_dynamic(buffer)
    }

    /// Encodes staticly-sized part of `Self`.
    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error>;

    /// Encodes dynamically-sized part of `Self`.
    /// The default implementation does nothing. Dynamically-sized contains should
    /// override this.
    fn encode_dynamic<O: Output + ?Sized>(&self, _buffer: &mut O) -> Result<(), Error> {
        Ok(())
    }

    /// Encodes `Self` into bytes vector. Required known size.
    #[cfg(feature = "alloc")]
    fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(self.size());
        self.encode(&mut vec).expect("Unable to encode self");
        vec
    }
}

/// Allows reading of data into a slice.
pub trait Input {
    /// Returns the remaining length of the input data.
    fn remaining(&mut self) -> usize;

    /// Peek the exact number of bytes required to fill the given buffer.
    fn peek(&self, buf: &mut [u8]) -> Result<(), Error>;

    /// Read the exact number of bytes required to fill the given buffer.
    fn read(&mut self, buf: &mut [u8]) -> Result<(), Error>;

    /// Peek a single byte from the input.
    fn peek_byte(&mut self) -> Result<u8, Error> {
        let mut buf = [0u8];
        self.peek(&mut buf[..])?;
        Ok(buf[0])
    }

    /// Read a single byte from the input.
    fn read_byte(&mut self) -> Result<u8, Error> {
        let mut buf = [0u8];
        self.read(&mut buf[..])?;
        Ok(buf[0])
    }

    /// Skips next `n` bytes.
    fn skip(&mut self, n: usize) -> Result<(), Error>;
}

/// Allows deserialize the type from the `Input`.
/// https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/tx_format.md#transaction
pub trait Deserialize: Sized {
    /// !INTERNAL USAGE ONLY!
    /// Array of bytes that are now aligned by themselves.
    #[doc(hidden)]
    const UNALIGNED_BYTES: bool = false;

    /// Decodes `Self` from the `buffer`.
    ///
    /// It is better to not implement this function directly, instead implement
    /// `decode_static` and `decode_dynamic`.
    fn decode<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
        let mut object = Self::decode_static(buffer)?;
        object.decode_dynamic(buffer)?;
        Ok(object)
    }

    /// Decodes static part of `Self` from the `buffer`.
    fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error>;

    /// Decodes dynamic part of the information from the `buffer` to fill `Self`.
    /// The default implementation does nothing. Dynamically-sized contains should
    /// override this.
    fn decode_dynamic<I: Input + ?Sized>(
        &mut self,
        _buffer: &mut I,
    ) -> Result<(), Error> {
        Ok(())
    }

    /// Helper method for deserializing `Self` from bytes.
    fn from_bytes(mut buffer: &[u8]) -> Result<Self, Error> {
        Self::decode(&mut buffer)
    }
}

/// Allows deserializing types while gracefully handling unknown fields for forward
/// compatibility. Returns both the deserialized object and metadata about what was
/// skipped.
///
/// This trait enables forward-compatible deserialization where older code can deserialize
/// data containing newer, unknown fields. This is particularly useful for:
/// - Protocol versioning (e.g., transaction policies with new policy types)
/// - Backward-compatible clients that need to handle future data formats
/// - Distributed systems where different nodes may use different versions
///
/// Each type can define its own metadata structure via the associated type,
/// allowing flexibility in what information is tracked during deserialization.
///
/// # Example
///
/// ```
/// use fuel_types::canonical::{Deserialize, DeserializeForwardCompatible, Serialize, Error, Input};
///
/// #[derive(Debug, PartialEq)]
/// struct MyStruct {
///     known_field: u32,
/// }
///
/// #[derive(Debug, Default)]
/// struct MyMetadata {
///     had_unknown_data: bool,
/// }
///
/// impl DeserializeForwardCompatible for MyStruct {
///     type Metadata = MyMetadata;
///
///     fn decode_static_forward_compatible<I: Input + ?Sized>(
///         buffer: &mut I,
///     ) -> Result<(Self, Self::Metadata), Error> {
///         let known_field = u32::decode(buffer)?;
///         let metadata = MyMetadata { had_unknown_data: false };
///         Ok((Self { known_field }, metadata))
///     }
/// }
/// ```
pub trait DeserializeForwardCompatible: Sized {
    /// Metadata type that tracks information about unknown/skipped fields
    /// during forward-compatible deserialization.
    type Metadata: Default;

    /// !INTERNAL USAGE ONLY!
    #[doc(hidden)]
    const UNALIGNED_BYTES: bool = false;

    /// Decodes `Self` from the buffer, gracefully handling unknown data.
    /// Returns the decoded object and metadata about what was skipped/unknown.
    fn decode_forward_compatible<I: Input + ?Sized>(
        buffer: &mut I,
    ) -> Result<(Self, Self::Metadata), Error> {
        let (mut object, mut metadata) = Self::decode_static_forward_compatible(buffer)?;
        object.decode_dynamic_forward_compatible(buffer, &mut metadata)?;
        Ok((object, metadata))
    }

    /// Decodes static part with forward compatibility, returning the object and initial
    /// metadata.
    fn decode_static_forward_compatible<I: Input + ?Sized>(
        buffer: &mut I,
    ) -> Result<(Self, Self::Metadata), Error>;

    /// Decodes dynamic part with forward compatibility, potentially updating the
    /// metadata. The default implementation does nothing. Dynamically-sized
    /// containers should override this.
    fn decode_dynamic_forward_compatible<I: Input + ?Sized>(
        &mut self,
        _buffer: &mut I,
        _metadata: &mut Self::Metadata,
    ) -> Result<(), Error> {
        Ok(())
    }

    /// Helper method for forward-compatible deserialization from bytes.
    fn from_bytes_forward_compatible(
        mut buffer: &[u8],
    ) -> Result<(Self, Self::Metadata), Error> {
        Self::decode_forward_compatible(&mut buffer)
    }
}

/// The data of each field should be aligned to 64 bits.
pub const ALIGN: usize = 8;

/// The number of padding bytes required to align the given length correctly.
#[allow(clippy::arithmetic_side_effects)] // Safety: (a % b) < b
const fn alignment_bytes(len: usize) -> usize {
    let modulo = len % ALIGN;
    if modulo == 0 { 0 } else { ALIGN - modulo }
}

/// Size after alignment. Saturates on overflow.
pub const fn aligned_size(len: usize) -> usize {
    len.saturating_add(alignment_bytes(len))
}

macro_rules! impl_for_primitives {
    ($t:ident, $unpadded:literal) => {
        impl Serialize for $t {
            const UNALIGNED_BYTES: bool = $unpadded;

            #[inline(always)]
            fn size_static(&self) -> usize {
                aligned_size(::core::mem::size_of::<$t>())
            }

            #[inline(always)]
            fn size_dynamic(&self) -> usize {
                0
            }

            #[inline(always)]
            fn encode_static<O: Output + ?Sized>(
                &self,
                buffer: &mut O,
            ) -> Result<(), Error> {
                // Primitive types are zero-padded on left side to a 8-byte boundary.
                // The resulting value is always well-aligned.
                let bytes = <$t>::to_be_bytes(*self);
                for _ in 0..alignment_bytes(bytes.len()) {
                    // Zero-pad
                    buffer.push_byte(0)?;
                }
                buffer.write(bytes.as_ref())?;
                Ok(())
            }
        }

        impl Deserialize for $t {
            const UNALIGNED_BYTES: bool = $unpadded;

            fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
                let mut asset = [0u8; ::core::mem::size_of::<$t>()];
                buffer.skip(alignment_bytes(asset.len()))?; // Skip zero-padding
                buffer.read(asset.as_mut())?;
                Ok(<$t>::from_be_bytes(asset))
            }
        }
    };
}

impl_for_primitives!(u8, true);
impl_for_primitives!(u16, false);
impl_for_primitives!(u32, false);
impl_for_primitives!(usize, false);
impl_for_primitives!(u64, false);
impl_for_primitives!(u128, false);

// Empty tuple `()`, i.e. the unit type takes up no space.
impl Serialize for () {
    fn size_static(&self) -> usize {
        0
    }

    #[inline(always)]
    fn size_dynamic(&self) -> usize {
        0
    }

    #[inline(always)]
    fn encode_static<O: Output + ?Sized>(&self, _buffer: &mut O) -> Result<(), Error> {
        Ok(())
    }
}

impl Deserialize for () {
    fn decode_static<I: Input + ?Sized>(_buffer: &mut I) -> Result<Self, Error> {
        Ok(())
    }
}

/// To protect against malicious large inputs, vector size is limited when decoding.
pub const VEC_DECODE_LIMIT: usize = 100 * (1 << 20); // 100 MiB

#[cfg(feature = "alloc")]
impl<T: Serialize> Serialize for Vec<T> {
    fn size_static(&self) -> usize {
        8
    }

    #[inline(always)]
    fn size_dynamic(&self) -> usize {
        if T::UNALIGNED_BYTES {
            aligned_size(self.len())
        } else {
            aligned_size(
                self.iter()
                    .map(|e| e.size())
                    .reduce(usize::saturating_add)
                    .unwrap_or_default(),
            )
        }
    }

    #[inline(always)]
    // Encode only the size of the vector. Elements will be encoded in the
    // `encode_dynamic` method.
    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        if self.len() > VEC_DECODE_LIMIT {
            return Err(Error::AllocationLimit);
        }
        let len: u64 = self.len().try_into().expect("msg.len() > u64::MAX");
        len.encode(buffer)
    }

    fn encode_dynamic<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        // Bytes - Vec<u8> it a separate case without padding for each element.
        // It should padded at the end if is not % ALIGN
        if T::UNALIGNED_BYTES {
            // SAFETY: `UNALIGNED_BYTES` only set for `u8`.
            let bytes = unsafe { ::core::mem::transmute::<&Vec<T>, &Vec<u8>>(self) };
            buffer.write(bytes.as_slice())?;
            for _ in 0..alignment_bytes(self.len()) {
                buffer.push_byte(0)?;
            }
        } else {
            for e in self.iter() {
                e.encode(buffer)?;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl<T: Deserialize> Deserialize for Vec<T> {
    // Decode only the capacity of the vector. Elements will be decoded in the
    // `decode_dynamic` method. The capacity is needed for iteration there.
    fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
        let cap = u64::decode(buffer)?;
        let cap: usize = cap.try_into().map_err(|_| Error::AllocationLimit)?;
        if cap > VEC_DECODE_LIMIT {
            return Err(Error::AllocationLimit);
        }

        if T::UNALIGNED_BYTES {
            // SAFETY: `UNALIGNED_BYTES` only set for `u8`.
            let vec = unsafe {
                let vec = vec![0u8; cap];
                ::core::mem::transmute::<Vec<u8>, Vec<T>>(vec)
            };

            Ok(vec)
        } else {
            Ok(Vec::with_capacity(cap))
        }
    }

    fn decode_dynamic<I: Input + ?Sized>(&mut self, buffer: &mut I) -> Result<(), Error> {
        // Bytes - Vec<u8> it a separate case without unpadding for each element.
        // It should unpadded at the end if is not % ALIGN
        if T::UNALIGNED_BYTES {
            // SAFETY: `UNALIGNED_BYTES` implemented set for `u8`.
            let _self =
                unsafe { ::core::mem::transmute::<&mut Vec<T>, &mut Vec<u8>>(self) };
            buffer.read(_self.as_mut())?;
        } else {
            for _ in 0..self.capacity() {
                self.push(T::decode(buffer)?);
            }
        }

        if T::UNALIGNED_BYTES {
            buffer.skip(alignment_bytes(self.capacity()))?;
        }

        Ok(())
    }
}

impl<const N: usize, T: Serialize> Serialize for [T; N] {
    fn size_static(&self) -> usize {
        if T::UNALIGNED_BYTES {
            aligned_size(N)
        } else {
            aligned_size(
                self.iter()
                    .map(|e| e.size_static())
                    .reduce(usize::saturating_add)
                    .unwrap_or_default(),
            )
        }
    }

    #[inline(always)]
    fn size_dynamic(&self) -> usize {
        if T::UNALIGNED_BYTES {
            0
        } else {
            aligned_size(
                self.iter()
                    .map(|e| e.size_dynamic())
                    .reduce(usize::saturating_add)
                    .unwrap_or_default(),
            )
        }
    }

    #[inline(always)]
    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        // Bytes - [u8; N] it a separate case without padding for each element.
        // It should padded at the end if is not % ALIGN
        if T::UNALIGNED_BYTES {
            // SAFETY: `Type::U8` implemented only for `u8`.
            let bytes = unsafe { ::core::mem::transmute::<&[T; N], &[u8; N]>(self) };
            buffer.write(bytes.as_slice())?;
            for _ in 0..alignment_bytes(N) {
                buffer.push_byte(0)?;
            }
        } else {
            for e in self.iter() {
                e.encode_static(buffer)?;
            }
        }
        Ok(())
    }

    fn encode_dynamic<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        if !T::UNALIGNED_BYTES {
            for e in self.iter() {
                e.encode_dynamic(buffer)?;
            }
        }

        Ok(())
    }
}

impl<const N: usize, T: Deserialize> Deserialize for [T; N] {
    fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
        if T::UNALIGNED_BYTES {
            let mut bytes: [u8; N] = [0; N];
            buffer.read(bytes.as_mut())?;
            buffer.skip(alignment_bytes(N))?;
            let ref_typed: &[T; N] = unsafe { core::mem::transmute(&bytes) };
            let typed: [T; N] = unsafe { core::ptr::read(ref_typed) };
            Ok(typed)
        } else {
            // Spec doesn't say how to deserialize arrays with unaligned
            // primitives(as `u16`, `u32`, `usize`), so unpad them.
            // SAFETY: `uninit`` is an array of `MaybUninit`, which do not require
            // initialization
            let mut uninit: [MaybeUninit<T>; N] =
                unsafe { MaybeUninit::uninit().assume_init() };
            // The following line coerces the pointer to the array to a pointer
            // to the first array element which is equivalent.
            for i in 0..N {
                match T::decode_static(buffer) {
                    Err(e) => {
                        for item in uninit.iter_mut().take(i) {
                            // SAFETY: all elements up to index i (excluded have been
                            // initialised)
                            unsafe {
                                item.assume_init_drop();
                            }
                        }
                        return Err(e);
                    }
                    Ok(decoded) => {
                        // SAFETY: `uninit[i]` is a MaybeUninit which can be
                        // safely overwritten.
                        uninit[i].write(decoded);

                        // SAFETY: Point to the next element after every iteration.
                        // 		 We do this N times therefore this is safe.
                    }
                }
            }

            // SAFETY: All array elements have been initialized above.
            let init = uninit.map(|v| unsafe { v.assume_init() });
            Ok(init)
        }
    }

    fn decode_dynamic<I: Input + ?Sized>(&mut self, buffer: &mut I) -> Result<(), Error> {
        if !T::UNALIGNED_BYTES {
            for e in self.iter_mut() {
                e.decode_dynamic(buffer)?;
            }
        }

        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl Output for Vec<u8> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}

impl Output for &'_ mut [u8] {
    fn write(&mut self, from: &[u8]) -> Result<(), Error> {
        if from.len() > self.len() {
            return Err(Error::BufferIsTooShort);
        }
        let len = from.len();
        self[..len].copy_from_slice(from);
        // We need to reduce the inner slice by `len`, because we already filled them.
        let reduced = &mut self[len..];

        // Compiler is not clever enough to allow it.
        // https://stackoverflow.com/questions/25730586/how-can-i-create-my-own-data-structure-with-an-iterator-that-returns-mutable-ref
        *self = unsafe { &mut *(reduced as *mut [u8]) };
        Ok(())
    }
}

impl Input for &'_ [u8] {
    fn remaining(&mut self) -> usize {
        self.len()
    }

    fn peek(&self, into: &mut [u8]) -> Result<(), Error> {
        if into.len() > self.len() {
            return Err(Error::BufferIsTooShort);
        }

        let len = into.len();
        into.copy_from_slice(&self[..len]);
        Ok(())
    }

    fn read(&mut self, into: &mut [u8]) -> Result<(), Error> {
        if into.len() > self.len() {
            return Err(Error::BufferIsTooShort);
        }

        let len = into.len();
        into.copy_from_slice(&self[..len]);
        *self = &self[len..];
        Ok(())
    }

    fn skip(&mut self, n: usize) -> Result<(), Error> {
        if n > self.len() {
            return Err(Error::BufferIsTooShort);
        }

        *self = &self[n..];
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate<T: Serialize + Deserialize + Eq + core::fmt::Debug>(t: T) {
        let bytes = t.to_bytes();
        let t2 = T::from_bytes(&bytes).expect("Roundtrip failed");
        assert_eq!(t, t2);
        assert_eq!(t.to_bytes(), t2.to_bytes());

        let mut vec = Vec::new();
        t.encode_static(&mut vec).expect("Encode failed");
        assert_eq!(vec.len(), t.size_static());
    }

    fn validate_enum<T: Serialize + Deserialize + Eq + fmt::Debug>(t: T) {
        let bytes = t.to_bytes();
        let t2 = T::from_bytes(&bytes).expect("Roundtrip failed");
        assert_eq!(t, t2);
        assert_eq!(t.to_bytes(), t2.to_bytes());

        let mut vec = Vec::new();
        t.encode_static(&mut vec).expect("Encode failed");
        assert_eq!(vec.len(), t.size_static());
        t.encode_dynamic(&mut vec).expect("Encode failed");
        assert_eq!(vec.len(), t.size());

        let mut vec2 = Vec::new();
        t.encode_dynamic(&mut vec2).expect("Encode failed");
        assert_eq!(vec2.len(), t.size_dynamic());
    }

    #[test]
    fn test_canonical_encode_decode() {
        validate(());
        validate(123u8);
        validate(u8::MAX);
        validate(123u16);
        validate(u16::MAX);
        validate(123u32);
        validate(u32::MAX);
        validate(123u64);
        validate(u64::MAX);
        validate(123u128);
        validate(u128::MAX);
        validate(Vec::<u8>::new());
        validate(Vec::<u16>::new());
        validate(Vec::<u32>::new());
        validate(Vec::<u64>::new());
        validate(Vec::<u128>::new());
        validate(vec![1u8]);
        validate(vec![1u16]);
        validate(vec![1u32]);
        validate(vec![1u64]);
        validate(vec![1u128]);
        validate(vec![1u8, 2u8]);
        validate(vec![1u16, 2u16]);
        validate(vec![1u32, 2u32]);
        validate(vec![1u64, 2u64]);
        validate(vec![1u128, 2u128]);

        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
        struct TestStruct1 {
            a: u8,
            b: u16,
        }

        let t = TestStruct1 { a: 123, b: 456 };
        assert_eq!(t.size_static(), 16);
        assert_eq!(t.size(), 16);
        validate(t);

        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
        struct TestStruct2 {
            a: u8,
            v: Vec<u8>,
            b: u16,
            arr0: [u8; 0],
            arr1: [u8; 2],
            arr2: [u16; 3],
            arr3: [u64; 4],
        }

        validate(TestStruct2 {
            a: 123,
            v: vec![1, 2, 3],
            b: 456,
            arr0: [],
            arr1: [1, 2],
            arr2: [1, 2, u16::MAX],
            arr3: [0, 3, 1111, u64::MAX],
        });

        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
        #[repr(transparent)]
        struct TestStruct3([u8; 64]);

        let t = TestStruct3([1; 64]);
        assert_eq!(t.size_static(), 64);
        assert_eq!(t.size(), 64);
        validate(t);

        #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
        #[canonical(prefix = 1u64)]
        struct Prefixed1 {
            a: [u8; 3],
            b: Vec<u8>,
        }
        validate(Prefixed1 {
            a: [1, 2, 3],
            b: vec![4, 5, 6],
        });

        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
        #[repr(u8)]
        enum TestEnum1 {
            A,
            B,
            C = 0x13,
            D,
        }

        validate(TestEnum1::A);
        validate(TestEnum1::B);
        validate(TestEnum1::C);
        validate(TestEnum1::D);

        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
        enum TestEnum2 {
            A(u8),
            B([u8; 3]),
            C(Vec<u8>),
        }

        validate_enum(TestEnum2::A(2));
        validate_enum(TestEnum2::B([1, 2, 3]));
        validate_enum(TestEnum2::C(vec![1, 2, 3]));

        #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
        #[canonical(prefix = 2u64)]
        struct Prefixed2(u16);
        validate(Prefixed2(u16::MAX));

        assert_eq!(
            &Prefixed1 {
                a: [1, 2, 3],
                b: vec![4, 5]
            }
            .to_bytes()[..8],
            &[0u8, 0, 0, 0, 0, 0, 0, 1]
        );
        assert_eq!(
            Prefixed2(u16::MAX).to_bytes(),
            [0u8, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0xff, 0xff]
        );
    }

    #[test]
    fn test_forward_compatible_deserialization_with_simple_struct() {
        // Given A simple struct that tracks how many unknown fields were encountered
        #[derive(Debug, PartialEq, Eq)]
        struct SimpleStruct {
            known_value: u32,
        }

        #[derive(Debug, Default, PartialEq, Eq)]
        struct SimpleMetadata {
            unknown_count: usize,
        }

        impl DeserializeForwardCompatible for SimpleStruct {
            type Metadata = SimpleMetadata;

            fn decode_static_forward_compatible<I: Input + ?Sized>(
                buffer: &mut I,
            ) -> Result<(Self, Self::Metadata), Error> {
                let known_value = u32::decode(buffer)?;
                let metadata = SimpleMetadata { unknown_count: 0 };
                Ok((Self { known_value }, metadata))
            }
        }

        // When testing normal deserialization
        let mut data = vec![0u8; 8];
        data[4..8].copy_from_slice(&123u32.to_be_bytes());

        let (obj, metadata) = SimpleStruct::from_bytes_forward_compatible(&data)
            .expect("Forward-compatible deserialization should succeed");

        // Then
        assert_eq!(obj.known_value, 123);
        assert_eq!(metadata.unknown_count, 0);
    }

    #[test]
    fn test_forward_compatible_deserialization_with_bitflags() {
        // Given - Simulates a bitflag-based struct like Policies
        #[derive(Debug, PartialEq, Eq)]
        struct BitflagStruct {
            known_bits: u8,
            values: [u32; 2],
        }

        #[derive(Debug, Default, PartialEq, Eq)]
        struct BitflagMetadata {
            raw_bits: u8,
            unknown_bits: u8,
        }

        impl DeserializeForwardCompatible for BitflagStruct {
            type Metadata = BitflagMetadata;

            fn decode_static_forward_compatible<I: Input + ?Sized>(
                buffer: &mut I,
            ) -> Result<(Self, Self::Metadata), Error> {
                let raw_bits = u8::decode(buffer)?;

                // Only bits 0 and 1 are known
                const KNOWN_BITS: u8 = 0b0000_0011;
                let known_bits = raw_bits & KNOWN_BITS;
                let unknown_bits = raw_bits & !KNOWN_BITS;

                let metadata = BitflagMetadata {
                    raw_bits,
                    unknown_bits,
                };

                Ok((
                    Self {
                        known_bits,
                        values: [0, 0],
                    },
                    metadata,
                ))
            }

            fn decode_dynamic_forward_compatible<I: Input + ?Sized>(
                &mut self,
                _buffer: &mut I,
                _metadata: &mut Self::Metadata,
            ) -> Result<(), Error> {
                // Decode only the values for known bits
                for i in 0..2 {
                    if (self.known_bits & (1 << i)) != 0 {
                        self.values[i] = u32::decode(_buffer)?;
                    }
                }
                Ok(())
            }
        }

        // When - Test with unknown bits (bits 2 and 3 are unknown)
        let mut data = Vec::new();
        0b0000_1011u8.encode(&mut data).unwrap(); // bits 0, 1, and 3 set

        // Add values for bit 0 and bit 1
        let mut value_data = Vec::new();
        100u32.encode(&mut value_data).unwrap();
        200u32.encode(&mut value_data).unwrap();
        data.extend_from_slice(&value_data);

        let (obj, metadata) =
            BitflagStruct::from_bytes_forward_compatible(&data).expect("Should decode");

        // Then
        assert_eq!(obj.known_bits, 0b0000_0011); // Only bits 0 and 1 are known
        assert_eq!(obj.values[0], 100);
        assert_eq!(obj.values[1], 200);
        assert_eq!(metadata.raw_bits, 0b0000_1011);
        assert_eq!(metadata.unknown_bits, 0b0000_1000); // Bit 3 is unknown
    }

    #[test]
    fn test_forward_compatible_deserialization_with_dynamic_update() {
        #[derive(Debug, PartialEq, Eq)]
        struct DynamicStruct {
            count: u8,
            values: Vec<u16>,
        }

        #[derive(Debug, Default, PartialEq, Eq)]
        struct DynamicMetadata {
            static_unknown: bool,
            dynamic_skipped: usize,
        }

        impl DeserializeForwardCompatible for DynamicStruct {
            type Metadata = DynamicMetadata;

            fn decode_static_forward_compatible<I: Input + ?Sized>(
                buffer: &mut I,
            ) -> Result<(Self, Self::Metadata), Error> {
                let count = u8::decode(buffer)?;
                let metadata = DynamicMetadata {
                    static_unknown: false,
                    dynamic_skipped: 0,
                };
                Ok((
                    Self {
                        count,
                        values: Vec::new(),
                    },
                    metadata,
                ))
            }

            fn decode_dynamic_forward_compatible<I: Input + ?Sized>(
                &mut self,
                buffer: &mut I,
                metadata: &mut Self::Metadata,
            ) -> Result<(), Error> {
                // Decode only first 2 values, skip the rest
                let max_decode = core::cmp::min(self.count as usize, 2);

                for _ in 0..max_decode {
                    self.values.push(u16::decode(buffer)?);
                }

                // Skip remaining values
                let skipped = (self.count as usize).saturating_sub(max_decode);
                for _ in 0..skipped {
                    buffer.skip(aligned_size(core::mem::size_of::<u16>()))?;
                }

                metadata.dynamic_skipped = skipped;
                Ok(())
            }
        }

        // When - Test with 4 values but only decode 2
        let mut data = Vec::new();
        4u8.encode(&mut data).unwrap(); // count = 4

        let mut value_data = Vec::new();
        10u16.encode(&mut value_data).unwrap();
        20u16.encode(&mut value_data).unwrap();
        30u16.encode(&mut value_data).unwrap();
        40u16.encode(&mut value_data).unwrap();
        data.extend_from_slice(&value_data);

        let (obj, metadata) = DynamicStruct::from_bytes_forward_compatible(&data)
            .expect("Should decode with skipping");

        // Then
        assert_eq!(obj.count, 4);
        assert_eq!(obj.values.len(), 2);
        assert_eq!(obj.values, vec![10, 20]);
        assert_eq!(metadata.dynamic_skipped, 2);
    }

    #[test]
    fn test_forward_compatible_vs_strict_deserialization() {
        // A type that implements both Deserialize (strict) and
        // DeserializeForwardCompatible
        #[derive(Debug, PartialEq, Eq)]
        struct DualStruct {
            value: u32,
            flag: u8,
        }

        #[derive(Debug, Default, PartialEq, Eq)]
        struct DualMetadata {
            has_extra_data: bool,
        }

        impl Serialize for DualStruct {
            fn size_static(&self) -> usize {
                aligned_size(core::mem::size_of::<u32>())
                    + aligned_size(core::mem::size_of::<u8>())
            }

            fn size_dynamic(&self) -> usize {
                0
            }

            fn encode_static<O: Output + ?Sized>(
                &self,
                buffer: &mut O,
            ) -> Result<(), Error> {
                self.value.encode(buffer)?;
                self.flag.encode(buffer)?;
                Ok(())
            }
        }

        impl Deserialize for DualStruct {
            fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
                let value = u32::decode(buffer)?;
                let flag = u8::decode(buffer)?;

                // Strict: reject if flag has unknown bits
                if flag > 0b0000_0001 {
                    return Err(Error::Unknown("Unknown flag bits"));
                }

                Ok(Self { value, flag })
            }
        }

        impl DeserializeForwardCompatible for DualStruct {
            type Metadata = DualMetadata;

            fn decode_static_forward_compatible<I: Input + ?Sized>(
                buffer: &mut I,
            ) -> Result<(Self, Self::Metadata), Error> {
                let value = u32::decode(buffer)?;
                let flag = u8::decode(buffer)?;

                let has_extra_data = flag > 0b0000_0001;
                let masked_flag = flag & 0b0000_0001;

                Ok((
                    Self {
                        value,
                        flag: masked_flag,
                    },
                    DualMetadata { has_extra_data },
                ))
            }
        }

        // When - Create data with unknown flag bits
        let mut data = Vec::new();
        100u32.encode(&mut data).unwrap();
        0b0000_1010u8.encode(&mut data).unwrap(); // Unknown bits set

        // Then - Strict deserialization should fail
        let strict_result = DualStruct::from_bytes(&data);
        assert!(strict_result.is_err());
        assert_eq!(
            strict_result.unwrap_err(),
            Error::Unknown("Unknown flag bits")
        );

        // Then - Forward-compatible deserialization should succeed
        let (obj, metadata) =
            DualStruct::from_bytes_forward_compatible(&data).expect("should succeed");

        assert_eq!(obj.value, 100);
        assert_eq!(obj.flag, 0); // Masked to known bits
        assert!(metadata.has_extra_data);
    }
}

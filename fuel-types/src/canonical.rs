//! Canonical serialization and deserialization of Fuel types.

#![allow(unsafe_code)]

use crate::{
    Address,
    AssetId,
    Bytes20,
    Bytes32,
    Bytes4,
    Bytes8,
    ContractId,
    MessageId,
    Nonce,
    Salt,
};
use alloc::vec::Vec;
use core::mem::MaybeUninit;
pub use fuel_derive::{
    Deserialize,
    Serialize,
};

/// Error when serializing or deserializing.
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    /// The data of each field should be 64 bits aligned.
    IsNotAligned,
    /// The buffer is to short for writing or reading.
    BufferIsTooShort,
    /// Got unknown enum's discriminant.
    UnknownDiscriminant,
    /// Wrong align.
    WrongAlign,
    /// Unknown error.
    Unknown(&'static str),
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

/// !INTERNAL USAGE ONLY!
/// This enum provides type information required for specialization and deserialization.
#[derive(Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Type {
    U8,
    U16,
    U32,
    USIZE,
    U64,
    U128,
    Unknown,
}

/// Allows serialize the type into the `Output`.
/// https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/tx_format.md#transaction
pub trait Serialize {
    // !INTERNAL USAGE ONLY!
    #[doc(hidden)]
    const TYPE: Type = Type::Unknown;

    /// Size of static portion of the type in bytes.
    const SIZE_STATIC: usize;
    /// True if the size has no dynamically sized fields.
    /// This implies that `SIZE_STATIC` is the full size of the type.
    const SIZE_NO_DYNAMIC: bool;

    /// Returns the size required for serialization of static data.
    ///
    /// # Note: This function has the performance of constants because,
    /// during compilation, the compiler knows all static sizes.
    fn size_static(&self) -> usize {
        let mut calculator = SizeCalculator(0);
        self.encode_static(&mut calculator)
            .expect("Can't encode to get a static size");
        calculator.size()
    }

    /// Returns the size required for serialization of dynamic data.
    fn size_dynamic(&self) -> usize {
        let mut calculator = SizeCalculator(0);
        self.encode_dynamic(&mut calculator)
            .expect("Can't encode to get a dynamic size");
        calculator.size()
    }

    /// Returns the size required for serialization of `Self`.
    fn size(&self) -> usize {
        self.size_static() + self.size_dynamic()
    }

    /// Encodes `Self` into bytes vector.
    fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(self.size());
        self.encode(&mut vec).expect("Unable to encode self");
        vec
    }

    /// Encodes `Self` into the `buffer`.
    ///
    /// It is better to not implement this function directly, instead implement
    /// `encode_static` and `encode_dynamic`.
    fn encode<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        self.encode_static(buffer)?;
        self.encode_dynamic(buffer)
    }

    /// Encodes static data, required for `Self` deserialization, into the `buffer`.
    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error>;

    /// Encodes dynamic information required to fill `Self` during deserialization.
    ///
    /// # Note: It is empty for primitives. But it can be helpful for containers because this
    /// method is called at the end of struct/enum serialization.
    fn encode_dynamic<O: Output + ?Sized>(&self, _buffer: &mut O) -> Result<(), Error> {
        Ok(())
    }
}

/// Allows reading of data into a slice.
pub trait Input: Clone {
    /// Returns the remaining length of the input data.
    fn remaining(&mut self) -> usize;

    /// Read the exact number of bytes required to fill the given buffer.
    fn read(&mut self, buf: &mut [u8]) -> Result<(), Error>;

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
    // !INTERNAL USAGE ONLY!
    #[doc(hidden)]
    const TYPE: Type = Type::Unknown;

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
    ///
    /// # Note: It is empty for primitives. But it can be helpful for containers to fill elements.
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

/// The data of each field should be 64 bits aligned.
pub const ALIGN: usize = 8;

/// Returns the number of bytes to fill aligned
const fn alignment_bytes(len: usize) -> usize {
    (ALIGN - (len % ALIGN)) % ALIGN
}

/// Size after alignment
pub const fn aligned_size(len: usize) -> usize {
    len + alignment_bytes(len)
}

macro_rules! impl_for_fuel_types {
    ($t:ident) => {
        impl Serialize for $t {
            const SIZE_NO_DYNAMIC: bool = true;
            // Fuel-types are transparent single-field structs, so the size matches
            const SIZE_STATIC: usize = aligned_size(::core::mem::size_of::<$t>());

            #[inline(always)]
            fn encode_static<O: Output + ?Sized>(
                &self,
                buffer: &mut O,
            ) -> Result<(), Error> {
                for _ in 0..alignment_bytes(self.as_ref().len()) {
                    buffer.push_byte(0)?;
                }
                buffer.write(self.as_ref())?;
                Ok(())
            }
        }

        impl Deserialize for $t {
            fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
                let mut asset = $t::zeroed();
                buffer.skip(alignment_bytes(asset.as_ref().len()))?;
                buffer.read(asset.as_mut())?;
                Ok(asset)
            }
        }
    };
}

impl_for_fuel_types!(Address);
impl_for_fuel_types!(AssetId);
impl_for_fuel_types!(ContractId);
impl_for_fuel_types!(Bytes4);
impl_for_fuel_types!(Bytes8);
impl_for_fuel_types!(Bytes20);
impl_for_fuel_types!(Bytes32);
impl_for_fuel_types!(MessageId);
impl_for_fuel_types!(Salt);
impl_for_fuel_types!(Nonce);

macro_rules! impl_for_primitives {
    ($t:ident, $ty:path) => {
        impl Serialize for $t {
            const SIZE_NO_DYNAMIC: bool = true;
            const SIZE_STATIC: usize = aligned_size(::core::mem::size_of::<$t>());
            const TYPE: Type = $ty;

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
            const TYPE: Type = $ty;

            fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
                let mut asset = [0u8; ::core::mem::size_of::<$t>()];
                buffer.skip(alignment_bytes(asset.len()))?; // Skip zero-padding
                buffer.read(asset.as_mut())?;
                println!(
                    "Deserialized {}: {}",
                    stringify!($t),
                    <$t>::from_be_bytes(asset)
                );
                println!("Remaining buffer: {}", buffer.remaining());
                Ok(<$t>::from_be_bytes(asset))
            }
        }
    };
}

impl_for_primitives!(u8, Type::U8);
impl_for_primitives!(u16, Type::U16);
impl_for_primitives!(u32, Type::U32);
impl_for_primitives!(usize, Type::USIZE); // TODO: encode as u64
impl_for_primitives!(u64, Type::U64);
impl_for_primitives!(u128, Type::U128);

// Empty tuple `()`, i.e. the unit type takes up no space.
impl Serialize for () {
    const SIZE_NO_DYNAMIC: bool = true;
    const SIZE_STATIC: usize = 0;

    #[inline(always)]
    fn size_static(&self) -> usize {
        0
    }

    #[inline(always)]
    fn size_dynamic(&self) -> usize {
        0
    }

    #[inline(always)]
    fn size(&self) -> usize {
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

impl<T: Serialize> Serialize for Vec<T> {
    const SIZE_NO_DYNAMIC: bool = false;
    const SIZE_STATIC: usize = 8;

    #[inline(always)]
    // Encode only the size of the vector. Elements will be encoded in the
    // `encode_dynamic` method.
    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        self.len().encode(buffer)
    }

    fn encode_dynamic<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        // Bytes - Vec<u8> it a separate case without padding for each element.
        // It should padded at the end if is not % ALIGN
        match T::TYPE {
            Type::U8 => {
                // SAFETY: `Type::U8` implemented only for `u8`.
                let bytes = unsafe { ::core::mem::transmute::<&Vec<T>, &Vec<u8>>(self) };
                buffer.write(bytes.as_slice())?;
                for _ in 0..alignment_bytes(self.len()) {
                    buffer.push_byte(0)?;
                }
            }
            _ => {
                for e in self.iter() {
                    e.encode(buffer)?;
                }
            }
        };

        Ok(())
    }
}

impl<T: Deserialize> Deserialize for Vec<T> {
    // Decode only the capacity of the vector. Elements will be decoded in the
    // `decode_dynamic` method. The capacity is needed for iteration there.
    fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
        let cap: usize = usize::decode(buffer)?;
        println!("Allocating vec with cap {}", cap);
        // TODO: this can panic with over-large capacity, and likely has to be reworked
        Ok(Vec::with_capacity(cap))
    }

    fn decode_dynamic<I: Input + ?Sized>(&mut self, buffer: &mut I) -> Result<(), Error> {
        println!("Remaining buffer: {}", buffer.remaining());
        println!("Restoring vec cap {}", self.capacity());

        for _ in 0..self.capacity() {
            // Bytes - Vec<u8> it a separate case without unpadding for each element.
            // It should unpadded at the end if is not % ALIGN
            match T::TYPE {
                Type::U8 => {
                    let byte = buffer.read_byte()?;
                    // SAFETY: `Type::U8` implemented only for `u8`, so it is `Vec<u8>`.
                    let _self = unsafe {
                        ::core::mem::transmute::<&mut Vec<T>, &mut Vec<u8>>(self)
                    };
                    _self.push(byte);
                }
                _ => {
                    self.push(T::decode(buffer)?);
                }
            };
        }

        if let Type::U8 = T::TYPE {
            buffer.skip(alignment_bytes(self.capacity()))?;
        }

        Ok(())
    }
}

impl<const N: usize, T: Serialize> Serialize for [T; N] {
    const SIZE_NO_DYNAMIC: bool = true;
    const SIZE_STATIC: usize = aligned_size(::core::mem::size_of::<T>()) * N;

    #[inline(always)]
    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        // Bytes - [u8; N] it a separate case without padding for each element.
        // It should padded at the end if is not % ALIGN
        match T::TYPE {
            Type::U8 => {
                // SAFETY: `Type::U8` implemented only for `u8`.
                let bytes = unsafe { ::core::mem::transmute::<&[T; N], &[u8; N]>(self) };
                buffer.write(bytes.as_slice())?;
                for _ in 0..alignment_bytes(N) {
                    buffer.push_byte(0)?;
                }
            }
            _ => {
                for e in self.iter() {
                    e.encode_static(buffer)?;
                }
            }
        };

        Ok(())
    }

    fn encode_dynamic<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        // All primitives have only static part, so skip dynamic encoding for them.
        if let Type::Unknown = T::TYPE {
            for e in self.iter() {
                e.encode_dynamic(buffer)?;
            }
        }

        Ok(())
    }
}

impl<const N: usize, T: Deserialize> Deserialize for [T; N] {
    fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
        match T::TYPE {
            Type::U8 => {
                let mut bytes: [u8; N] = [0; N];
                buffer.read(bytes.as_mut())?;
                buffer.skip(alignment_bytes(N))?;
                let ref_typed: &[T; N] = unsafe { core::mem::transmute(&bytes) };
                let typed: [T; N] = unsafe { core::ptr::read(ref_typed) };
                Ok(typed)
            }
            // Spec doesn't say how to deserialize arrays with unaligned
            // primitives(as `u16`, `u32`, `usize`), so unpad them.
            _ => {
                let mut uninit = <MaybeUninit<[T; N]>>::uninit();
                // The following line coerces the pointer to the array to a pointer
                // to the first array element which is equivalent.
                let mut ptr = uninit.as_mut_ptr() as *mut T;
                for _ in 0..N {
                    let decoded = T::decode_static(buffer)?;
                    // SAFETY: We do not read uninitialized array contents
                    // 		 while initializing them.
                    unsafe {
                        core::ptr::write(ptr, decoded);
                    }
                    // SAFETY: Point to the next element after every iteration.
                    // 		 We do this N times therefore this is safe.
                    ptr = unsafe { ptr.add(1) };
                }
                // SAFETY: All array elements have been initialized above.
                let init = unsafe { uninit.assume_init() };
                Ok(init)
            }
        }
    }

    fn decode_dynamic<I: Input + ?Sized>(&mut self, buffer: &mut I) -> Result<(), Error> {
        // All primitives have only static part, so skip dynamic decoding for them.
        if let Type::Unknown = T::TYPE {
            for e in self.iter_mut() {
                e.decode_dynamic(buffer)?;
            }
        }

        Ok(())
    }
}

impl Output for Vec<u8> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}

impl<'a> Output for &'a mut [u8] {
    fn write(&mut self, from: &[u8]) -> Result<(), Error> {
        if from.len() > self.len() {
            return Err(Error::BufferIsTooShort)
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

/// Counts the number of written bytes.
pub struct SizeCalculator(usize);

impl SizeCalculator {
    /// The number of written bytes.
    pub fn size(self) -> usize {
        self.0
    }
}

impl Output for SizeCalculator {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or(Error::BufferIsTooShort)?;
        Ok(())
    }
}

impl<'a> Input for &'a [u8] {
    fn remaining(&mut self) -> usize {
        self.len()
    }

    fn read(&mut self, into: &mut [u8]) -> Result<(), Error> {
        if into.len() > self.len() {
            return Err(Error::BufferIsTooShort)
        }

        let len = into.len();
        into.copy_from_slice(&self[..len]);
        *self = &self[len..];
        Ok(())
    }

    fn skip(&mut self, n: usize) -> Result<(), Error> {
        if n > self.len() {
            return Err(Error::BufferIsTooShort)
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
        assert_eq!(vec.len(), T::SIZE_STATIC);
    }

    #[test]
    fn xxx_yyy() {
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

        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
        struct TestStruct1 {
            a: u8,
            b: u16,
        }

        validate(TestStruct1 { a: 123, b: 456 });

        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
        struct TestStruct2 {
            a: u8,
            v: Vec<u8>,
            b: u16,
        }

        validate(TestStruct2 {
            a: 123,
            v: vec![1, 2, 3],
            b: 456,
        });

        // #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
        // enum TestEnum1 {
        //     A(u8),
        //     B(u16),
        // }

        // validate(TestEnum1::A(123));
        // validate(TestEnum1::B(456));
    }
}

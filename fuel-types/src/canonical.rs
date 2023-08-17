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
pub trait Input {
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
const fn fill_bytes(len: usize) -> usize {
    (ALIGN - (len % ALIGN)) % ALIGN
}

/// Writes zero bytes to fill alignment into the `buffer`.
macro_rules! align_during_encode {
    ($t:ty, $buffer:ident) => {
        // FIXME: This is unsound; size_of shouldn't affect the serialized size.
        //        The compiler is allowed to add arbitrary padding to structs.
        const FILL_SIZE: usize = fill_bytes(::core::mem::size_of::<$t>());
        // It will be removed by the compiler if `FILL_SIZE` is zero.
        if FILL_SIZE > 0 {
            let zeroed: [u8; FILL_SIZE] = [0; FILL_SIZE];
            $buffer.write(zeroed.as_ref())?;
        }
    };
}

/// Skips zero bytes added for alignment from the `buffer`.
macro_rules! align_during_decode {
    ($t:ident, $buffer:ident) => {
        // FIXME: This is unsound; size_of shouldn't affect the serialized size.
        //        The compiler is allowed to add arbitrary padding to structs.
        const FILL_SIZE: usize = fill_bytes(::core::mem::size_of::<$t>());
        // It will be removed by the compiler if `FILL_SIZE` is zero.
        if FILL_SIZE > 0 {
            $buffer.skip(FILL_SIZE)?;
        }
    };
}

macro_rules! impl_for_fuel_types {
    ($t:ident) => {
        impl Serialize for $t {
            #[inline(always)]
            fn encode_static<O: Output + ?Sized>(
                &self,
                buffer: &mut O,
            ) -> Result<(), Error> {
                buffer.write(self.as_ref())?;
                align_during_encode!($t, buffer);
                Ok(())
            }
        }

        impl Deserialize for $t {
            fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
                let mut asset = $t::zeroed();
                buffer.read(asset.as_mut())?;
                align_during_decode!($t, buffer);
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
            const TYPE: Type = $ty;

            #[inline(always)]
            fn encode_static<O: Output + ?Sized>(
                &self,
                buffer: &mut O,
            ) -> Result<(), Error> {
                let bytes = <$t>::to_be_bytes(*self);
                buffer.write(bytes.as_ref())?;
                align_during_encode!($t, buffer);
                Ok(())
            }
        }

        impl Deserialize for $t {
            const TYPE: Type = $ty;

            fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
                let mut asset = [0u8; ::core::mem::size_of::<$t>()];
                buffer.read(asset.as_mut())?;
                align_during_decode!($t, buffer);
                println!("Deserialized {}: {}", stringify!($t), <$t>::from_be_bytes(asset));
                println!("Remaining buffer: {}", buffer.remaining());
                Ok(<$t>::from_be_bytes(asset))
            }
        }
    };
}

impl_for_primitives!(u8, Type::U8);
impl_for_primitives!(u16, Type::U16);
impl_for_primitives!(u32, Type::U32);
impl_for_primitives!(usize, Type::USIZE);
impl_for_primitives!(u64, Type::U64);
impl_for_primitives!(u128, Type::U128);

// Empty tuple `()`, i.e. the unit type takes up no space.
impl Serialize for () {
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
                for _ in 0..fill_bytes(self.len()) {
                    buffer.push_byte(0)?;
                }
            }
            // Spec doesn't say how to serialize arrays with unaligned
            // primitives(as `u16`, `u32`, `usize`), so pad them.
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
                // Spec doesn't say how to deserialize arrays with unaligned
                // primitives(as `u16`, `u32`, `usize`), so unpad them.
                _ => {
                    self.push(T::decode(buffer)?);
                }
            };
        }

        if let Type::U8 = T::TYPE {
            buffer.skip(fill_bytes(self.capacity()))?;
        }

        Ok(())
    }
}

impl<const N: usize, T: Serialize> Serialize for [T; N] {
    #[inline(always)]
    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        // Bytes - [u8; N] it a separate case without padding for each element.
        // It should padded at the end if is not % ALIGN
        match T::TYPE {
            Type::U8 => {
                // SAFETY: `Type::U8` implemented only for `u8`.
                let bytes = unsafe { ::core::mem::transmute::<&[T; N], &[u8; N]>(self) };
                buffer.write(bytes.as_slice())?;
                for _ in 0..fill_bytes(N) {
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
                buffer.skip(fill_bytes(N))?;
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
mod test {
    use super::*;
    use itertools::Itertools;
    use rand::{
        rngs::StdRng,
        Rng,
        SeedableRng,
    };

    #[test]
    fn fuel_types_encode() {
        macro_rules! encode_with_empty_bytes {
            ($ty:path, $empty_bytes:expr, $t:expr, $s:expr) => {{
                let rng = &mut StdRng::seed_from_u64(8586);
                const NUMBER_OF_EMPTY_BYTES: usize = $empty_bytes;
                assert_eq!(<$ty as Serialize>::TYPE, $t);

                for _ in 0..1000 {
                    let fuel_type: $ty = rng.gen();
                    // Spec says: as-is, with padding zeroes aligned to 8 bytes.
                    // https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/tx_format.md#transaction
                    let expected_bytes: Vec<u8> =
                        [fuel_type.as_ref(), [0u8; NUMBER_OF_EMPTY_BYTES].as_slice()].concat();

                    let actual_bytes = fuel_type.to_bytes();
                    assert_eq!(actual_bytes.len(), expected_bytes.len());
                    assert_eq!(actual_bytes.len(), <$ty>::LEN + NUMBER_OF_EMPTY_BYTES);
                    assert_eq!(actual_bytes.as_slice(), expected_bytes.as_slice());
                    assert_eq!(Serialize::size(&fuel_type), $s);
                    assert_eq!(Serialize::size_static(&fuel_type), $s);
                    assert_eq!(Serialize::size_dynamic(&fuel_type), 0);
                }
            }};
        }

        // Types are aligned by default.
        encode_with_empty_bytes!(Address, 0, Type::Unknown, 32);
        encode_with_empty_bytes!(AssetId, 0, Type::Unknown, 32);
        encode_with_empty_bytes!(ContractId, 0, Type::Unknown, 32);
        encode_with_empty_bytes!(Bytes8, 0, Type::Unknown, 8);
        encode_with_empty_bytes!(Bytes32, 0, Type::Unknown, 32);
        encode_with_empty_bytes!(MessageId, 0, Type::Unknown, 32);
        encode_with_empty_bytes!(Salt, 0, Type::Unknown, 32);

        // Types are not aligned by default.
        encode_with_empty_bytes!(Bytes4, 4, Type::Unknown, 8);
        encode_with_empty_bytes!(Bytes20, 4, Type::Unknown, 24);

        assert_eq!(
            hex::encode(<Bytes4 as Serialize>::to_bytes(&[0xFF; 4].into())),
            "ffffffff00000000"
        );
        assert_eq!(
            hex::encode(<Bytes20 as Serialize>::to_bytes(&[0xFF; 20].into())),
            "ffffffffffffffffffffffffffffffffffffffff00000000"
        );
    }

    #[test]
    fn fuel_types_decode() {
        macro_rules! decode_with_empty_bytes {
            ($ty:path, $empty_bytes:expr) => {{
                let rng = &mut StdRng::seed_from_u64(8586);
                const NUMBER_OF_EMPTY_BYTES: usize = $empty_bytes;

                for _ in 0..1000 {
                    let expected_bytes: [u8; <$ty>::LEN] = rng.gen();
                    let mut actual_bytes: Vec<u8> = [
                        expected_bytes.as_slice(),
                        [0u8; NUMBER_OF_EMPTY_BYTES].as_slice(),
                    ]
                    .concat();

                    assert_eq!(actual_bytes.len(), <$ty>::LEN + NUMBER_OF_EMPTY_BYTES);

                    let fuel_type: $ty = <$ty>::decode(&mut actual_bytes.as_slice())
                        .expect("Unable to decode");
                    assert_eq!(fuel_type.as_ref(), expected_bytes.as_ref());

                    // Remove last byte to force error during decoding
                    actual_bytes.pop();
                    assert_eq!(
                        actual_bytes.len(),
                        <$ty>::LEN + NUMBER_OF_EMPTY_BYTES - 1
                    );
                    assert_eq!(
                        <$ty>::decode(&mut actual_bytes.as_slice()),
                        Err(Error::BufferIsTooShort)
                    );
                }
            }};
        }

        // Types are aligned by default.
        decode_with_empty_bytes!(Address, 0);
        decode_with_empty_bytes!(AssetId, 0);
        decode_with_empty_bytes!(ContractId, 0);
        decode_with_empty_bytes!(Bytes8, 0);
        decode_with_empty_bytes!(Bytes32, 0);
        decode_with_empty_bytes!(MessageId, 0);
        decode_with_empty_bytes!(Salt, 0);

        // Types are not aligned by default.
        decode_with_empty_bytes!(Bytes4, 4);
        decode_with_empty_bytes!(Bytes20, 4);
    }

    #[test]
    fn primitives_encode() {
        macro_rules! encode_with_empty_bytes {
            ($ty:path, $empty_bytes:expr, $t:expr, $s:expr) => {{
                let rng = &mut StdRng::seed_from_u64(8586);
                const NUMBER_OF_EMPTY_BYTES: usize = $empty_bytes;
                assert_eq!(<$ty as Serialize>::TYPE, $t);

                for _ in 0..1000 {
                    let primitive: $ty = rng.gen();
                    // Spec says: big-endian right-aligned to 8 bytes
                    // https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/tx_format.md#transaction
                    let expected_bytes: Vec<u8> = [
                        primitive.to_be_bytes().as_ref(),
                        [0u8; NUMBER_OF_EMPTY_BYTES].as_slice(),
                    ]
                    .concat();

                    let actual_bytes = primitive.to_bytes();
                    assert_eq!(actual_bytes.len(), expected_bytes.len());
                    assert_eq!(
                        actual_bytes.len(),
                        ::core::mem::size_of::<$ty>() + NUMBER_OF_EMPTY_BYTES
                    );
                    assert_eq!(actual_bytes.as_slice(), expected_bytes.as_slice());
                    assert_eq!(Serialize::size(&primitive), $s);
                    assert_eq!(Serialize::size_static(&primitive), $s);
                    assert_eq!(Serialize::size_dynamic(&primitive), 0);
                }
            }};
        }

        // Types are aligned by default.
        encode_with_empty_bytes!(u64, 0, Type::U64, 8);
        encode_with_empty_bytes!(u128, 0, Type::U128, 16);
        encode_with_empty_bytes!(usize, 0, Type::USIZE, 8);

        // Types are not aligned by default.
        encode_with_empty_bytes!(u8, 7, Type::U8, 8);
        encode_with_empty_bytes!(u16, 6, Type::U16, 8);
        encode_with_empty_bytes!(u32, 4, Type::U32, 8);

        assert_eq!(
            hex::encode(Serialize::to_bytes(&0xFFu8)),
            "ff00000000000000"
        );
        assert_eq!(
            hex::encode(Serialize::to_bytes(&0xFFu16)),
            "00ff000000000000"
        );
        assert_eq!(
            hex::encode(Serialize::to_bytes(&0xFFu32)),
            "000000ff00000000"
        );
        assert_eq!(
            hex::encode(Serialize::to_bytes(&0xFFu64)),
            "00000000000000ff"
        );
        assert_eq!(
            hex::encode(Serialize::to_bytes(&0xFFusize)),
            "00000000000000ff"
        );
        assert_eq!(
            hex::encode(Serialize::to_bytes(&0xFFu128)),
            "000000000000000000000000000000ff"
        );
    }

    #[test]
    fn primitives_decode() {
        macro_rules! decode_with_empty_bytes {
            ($ty:path, $empty_bytes:expr) => {{
                let rng = &mut StdRng::seed_from_u64(8586);
                const NUMBER_OF_EMPTY_BYTES: usize = $empty_bytes;

                for _ in 0..1000 {
                    let expected_bytes: [u8; ::core::mem::size_of::<$ty>()] = rng.gen();
                    let mut actual_bytes: Vec<u8> = [
                        expected_bytes.as_slice(),
                        [0u8; NUMBER_OF_EMPTY_BYTES].as_slice(),
                    ]
                    .concat();

                    assert_eq!(
                        actual_bytes.len(),
                        ::core::mem::size_of::<$ty>() + NUMBER_OF_EMPTY_BYTES
                    );

                    let primitive: $ty = <$ty>::decode(&mut actual_bytes.as_slice())
                        .expect("Unable to decode");
                    assert_eq!(primitive.to_be_bytes().as_ref(), expected_bytes.as_ref());

                    // Remove last byte to force error during decoding
                    actual_bytes.pop();
                    assert_eq!(
                        actual_bytes.len(),
                        ::core::mem::size_of::<$ty>() + NUMBER_OF_EMPTY_BYTES - 1
                    );
                    assert_eq!(
                        <$ty>::decode(&mut actual_bytes.as_slice()),
                        Err(Error::BufferIsTooShort)
                    );
                }
            }};
        }

        // Types are aligned by default.
        decode_with_empty_bytes!(u64, 0);
        decode_with_empty_bytes!(u128, 0);
        decode_with_empty_bytes!(usize, 0);

        // Types are not aligned by default.
        decode_with_empty_bytes!(u8, 7);
        decode_with_empty_bytes!(u16, 6);
        decode_with_empty_bytes!(u32, 4);
    }

    #[test]
    fn vector_encode_bytes() {
        macro_rules! encode_bytes {
            ($num:expr, $padding:expr) => {{
                let rng = &mut StdRng::seed_from_u64(8586);
                let mut bytes = Vec::with_capacity(1013);
                const NUM: usize = $num;
                const PADDING: usize = $padding;
                const PADDED_NUM: usize = NUM /* bytes */ + PADDING;
                for _ in 0..NUM {
                    bytes.push(rng.gen::<u8>())
                }
                assert_eq!(bytes.len(), NUM);

                // Correct sizes for each part
                assert_eq!(bytes.size_static(), 8);
                assert_eq!(bytes.size_dynamic(), PADDED_NUM);
                assert_eq!(bytes.size(), 8 /* static part */ + PADDED_NUM);

                // Correct encoding of static part
                let mut static_part = [0u8; 8];
                bytes
                    .encode_static(&mut static_part.as_mut())
                    .expect("Can't encode static part of bytes vector");
                assert_eq!(static_part.as_slice(), NUM.to_bytes().as_slice());

                // Correct encoding of dynamic part
                let mut dynamic_part = [0u8; PADDED_NUM];
                bytes
                    .encode_dynamic(&mut dynamic_part.as_mut())
                    .expect("Can't encode dynamic part of bytes vector");
                let expected_bytes = [bytes.as_slice(), [0u8; PADDING].as_slice()].concat();
                assert_eq!(dynamic_part.as_slice(), expected_bytes.as_slice());

                // Correct encoding
                let actual_bytes = bytes.to_bytes();
                let expected_bytes = [
                    NUM.to_bytes().as_slice(),
                    bytes.as_slice(),
                    [0u8; PADDING].as_slice(),
                ]
                .concat();
                assert_eq!(actual_bytes.len(), expected_bytes.len());
                assert_eq!(actual_bytes.as_slice(), expected_bytes.as_slice());
            }};
        }

        encode_bytes!(96, 0);
        encode_bytes!(97, 7);
        encode_bytes!(98, 6);
        encode_bytes!(99, 5);
        encode_bytes!(100, 4);
        encode_bytes!(101, 3);
        encode_bytes!(102, 2);
        encode_bytes!(103, 1);
        encode_bytes!(104, 0);

        assert_eq!(
            hex::encode(Serialize::to_bytes(&vec![0x11u8, 0x22u8, 0x33u8,])),
            "00000000000000031122330000000000"
        );
        assert_eq!(
            hex::encode(Serialize::to_bytes(&vec![
                0x11u8, 0x22u8, 0x33u8, 0x44u8, 0x55u8, 0x66u8,
            ])),
            "00000000000000061122334455660000"
        );
        assert_eq!(
            hex::encode(Serialize::to_bytes(&vec![
                0x11u8, 0x22u8, 0x33u8, 0x44u8, 0x55u8, 0x66u8, 0x77, 0x88,
            ])),
            "00000000000000081122334455667788"
        );
    }

    #[test]
    fn vector_decode_bytes() {
        macro_rules! decode_bytes {
            ($num:expr, $padding:expr) => {{
                let rng = &mut StdRng::seed_from_u64(8586);
                let mut bytes = Vec::with_capacity(1013);
                const NUM: usize = $num;
                const PADDING: usize = $padding;
                const PADDED_NUM: usize = NUM /* bytes */ + PADDING;
                NUM.encode(&mut bytes).expect("Should encode the size of the vector");
                let mut expected_bytes = vec![];
                for _ in 0..NUM {
                    let byte = rng.gen::<u8>();
                    bytes.push(byte);
                    expected_bytes.push(byte);
                }
                #[allow(clippy::reversed_empty_ranges)]
                for _ in 0..PADDING {
                    bytes.push(0);
                }
                assert_eq!(bytes.len(), 8 + PADDED_NUM);
                assert_eq!(expected_bytes.len(), NUM);

                // Correct decoding of static part
                let mut decoded = Vec::<u8>::decode_static(&mut bytes.as_slice())
                    .expect("Can't decode static part of bytes vector");
                assert_eq!(decoded.capacity(), NUM);
                assert_eq!(decoded.len(), 0);

                // Correct decoding of dynamic part
                decoded.decode_dynamic(&mut bytes[8..].as_ref())
                    .expect("Can't decode dynamic part of bytes vector");
                assert_eq!(decoded.len(), NUM);
                assert_eq!(decoded.as_slice(), expected_bytes.as_slice());

                // Correct decoding
                let decoded = Vec::<u8>::decode(&mut bytes.as_slice())
                    .expect("Can't decode of bytes vector");
                assert_eq!(decoded.len(), NUM);
                assert_eq!(decoded.as_slice(), expected_bytes.as_slice());

                // Pop last byte to cause an error during decoding
                bytes.pop();
                assert_eq!(bytes.len(), 8 + PADDED_NUM - 1);
                assert_eq!(Vec::<u8>::decode(&mut bytes.as_slice()), Err(Error::BufferIsTooShort));
            }};
        }

        decode_bytes!(96, 0);
        decode_bytes!(97, 7);
        decode_bytes!(98, 6);
        decode_bytes!(99, 5);
        decode_bytes!(100, 4);
        decode_bytes!(101, 3);
        decode_bytes!(102, 2);
        decode_bytes!(103, 1);
        decode_bytes!(104, 0);
    }

    #[test]
    fn vector_encode_decode_not_bytes() {
        macro_rules! encode_decode_not_bytes {
            ($ty:ty, $num:expr, $padding:expr) => {{
                let rng = &mut StdRng::seed_from_u64(8586);
                let mut vector = Vec::with_capacity(1013);
                // Total number of elements
                const NUM: usize = $num;
                // Padding per element in the vector
                const PADDING: usize = $padding;
                // Total encoded size with padding
                const PADDED_SIZE: usize = ::core::mem::size_of::<$ty>() * NUM + PADDING * NUM;
                for _ in 0..NUM {
                    vector.push(rng.gen::<$ty>())
                }
                assert_eq!(vector.len(), NUM);

                // Correct sizes for each part
                assert_eq!(vector.size_static(), 8);
                assert_eq!(vector.size_dynamic(), PADDED_SIZE);
                assert_eq!(vector.size(), 8 /* static part */ + PADDED_SIZE);

                // Correct encoding and decoding of static part
                let mut static_part = [0u8; 8];
                vector
                    .encode_static(&mut static_part.as_mut())
                    .expect("Can't encode static part of vector");
                assert_eq!(static_part.as_slice(), NUM.to_bytes().as_slice());
                let mut decoded = Vec::<$ty>::decode_static(&mut static_part.as_ref())
                    .expect("Can't decode static part of the vector");
                assert_eq!(decoded.capacity(), NUM);
                assert_eq!(decoded.len(), 0);

                // Correct encoding and decoding of dynamic part
                let mut dynamic_part = [0u8; PADDED_SIZE];
                vector
                    .encode_dynamic(&mut dynamic_part.as_mut())
                    .expect("Can't encode dynamic part of vector");
                let expected_bytes = vector.clone().into_iter()
                    .flat_map(|e| e.to_bytes().into_iter()).collect_vec();
                assert_eq!(dynamic_part.as_slice(), expected_bytes.as_slice());
                decoded.decode_dynamic(&mut dynamic_part.as_ref())
                    .expect("Can't decode dynamic part of the vector");
                assert_eq!(decoded.len(), NUM);
                assert_eq!(decoded.as_slice(), vector.as_slice());

                // Correct encoding and decoding
                let mut actual_bytes = vector.to_bytes();
                let expected_bytes = [
                    NUM.to_bytes().as_slice(),
                    vector.clone().into_iter()
                        .flat_map(|e| e.to_bytes().into_iter()).collect_vec().as_slice(),
                ]
                .concat();
                assert_eq!(actual_bytes.len(), expected_bytes.len());
                assert_eq!(actual_bytes.as_slice(), expected_bytes.as_slice());
                let decoded = Vec::<$ty>::decode(&mut actual_bytes.as_slice())
                    .expect("Can't decode the vector");
                assert_eq!(decoded.len(), vector.len());
                assert_eq!(decoded.as_slice(), vector.as_slice());

                // Pop last byte to cause an error during decoding
                actual_bytes.pop();
                assert_eq!(Vec::<$ty>::decode(&mut actual_bytes.as_slice()), Err(Error::BufferIsTooShort));
            }};
        }

        encode_decode_not_bytes!(Address, 100, 0);
        encode_decode_not_bytes!(AssetId, 100, 0);
        encode_decode_not_bytes!(ContractId, 100, 0);
        encode_decode_not_bytes!(Bytes4, 100, 4);
        encode_decode_not_bytes!(Bytes8, 100, 0);
        encode_decode_not_bytes!(Bytes20, 100, 4);
        encode_decode_not_bytes!(Bytes32, 100, 0);
        encode_decode_not_bytes!(MessageId, 100, 0);
        encode_decode_not_bytes!(Salt, 100, 0);

        encode_decode_not_bytes!(u16, 100, 6);
        encode_decode_not_bytes!(u32, 100, 4);
        encode_decode_not_bytes!(u64, 100, 0);
        encode_decode_not_bytes!(usize, 100, 0);
        encode_decode_not_bytes!(u128, 100, 0);

        assert_eq!(
            hex::encode(Serialize::to_bytes(&vec![
                Bytes4::new([0x11u8, 0x22u8, 0x33u8, 0x44u8]),
                Bytes4::zeroed(),
                Bytes4::new([0x11u8, 0x22u8, 0x33u8, 0x44u8])
            ])),
            "0000000000000003112233440000000000000000000000001122334400000000"
        );

        assert_eq!(
            hex::encode(Serialize::to_bytes(&vec![
                0xAAu16, 0xBBu16, 0xCCu16, 0xDDu16,
            ])),
            "000000000000000400aa00000000000000bb00000000000000cc00000000000000dd000000000000"
        );
    }

    #[test]
    fn vector_encode_decode_recursion() {
        macro_rules! encode_decode_recursion {
            ($ty:ty, $num:expr, $padding:expr) => {{
                let rng = &mut StdRng::seed_from_u64(8586);
                let mut vector: Vec<Vec<Vec<$ty>>> = Vec::with_capacity(1013);
                // Total number of elements in each vector
                const NUM: usize = $num;
                // Padding per element in the final vector
                const PADDING: usize = $padding;
                // Total encoded size with padding
                const PADDED_SIZE: usize =
                    ::core::mem::size_of::<$ty>() * NUM + PADDING * NUM;
                const DYNAMIC_SIZE: usize =
                    (NUM + NUM * NUM) * 8 + NUM * NUM * PADDED_SIZE;
                for _ in 0..NUM {
                    let mut first = Vec::with_capacity(1013);
                    for _ in 0..NUM {
                        let mut second = Vec::with_capacity(1013);
                        for _ in 0..NUM {
                            second.push(rng.gen::<$ty>())
                        }
                        first.push(second);
                    }
                    vector.push(first);
                }
                assert_eq!(vector.len(), NUM);

                // Correct sizes for each part
                assert_eq!(vector.size_static(), 8);
                assert_eq!(vector.size_dynamic(), DYNAMIC_SIZE);
                assert_eq!(vector.size(), 8 + DYNAMIC_SIZE);

                // Correct encoding and decoding of static part
                let mut static_part = [0u8; 8];
                vector
                    .encode_static(&mut static_part.as_mut())
                    .expect("Can't encode static part of vector");
                assert_eq!(static_part.as_slice(), NUM.to_bytes().as_slice());
                let mut decoded =
                    Vec::<Vec<Vec<$ty>>>::decode_static(&mut static_part.as_ref())
                        .expect("Can't decode static part of the vector");
                assert_eq!(decoded.capacity(), NUM);
                assert_eq!(decoded.len(), 0);

                // Correct encoding and decoding of dynamic part
                let mut dynamic_part = [0u8; DYNAMIC_SIZE];
                vector
                    .encode_dynamic(&mut dynamic_part.as_mut())
                    .expect("Can't encode dynamic part of vector");
                let expected_bytes = vector
                    .clone()
                    .into_iter()
                    .flat_map(|e| e.to_bytes().into_iter())
                    .collect_vec();
                assert_eq!(dynamic_part.as_slice(), expected_bytes.as_slice());
                decoded
                    .decode_dynamic(&mut dynamic_part.as_ref())
                    .expect("Can't decode dynamic part of the vector");
                assert_eq!(decoded.len(), NUM);
                assert_eq!(decoded.as_slice(), vector.as_slice());

                for i in 0..NUM {
                    assert_eq!(decoded[i].len(), NUM);
                    assert_eq!(decoded[i].as_slice(), vector[i].as_slice());
                    for j in 0..NUM {
                        assert_eq!(decoded[i][j].len(), NUM);
                        assert_eq!(decoded[i][j].as_slice(), vector[i][j].as_slice());
                        for n in 0..NUM {
                            assert_eq!(decoded[i][j][n], vector[i][j][n]);
                        }
                    }
                }

                // Correct encoding and decoding
                let mut actual_bytes = vector.to_bytes();
                let expected_bytes = [
                    NUM.to_bytes().as_slice(),
                    vector
                        .clone()
                        .into_iter()
                        .flat_map(|e| e.to_bytes().into_iter())
                        .collect_vec()
                        .as_slice(),
                ]
                .concat();
                assert_eq!(actual_bytes.len(), expected_bytes.len());
                assert_eq!(actual_bytes.as_slice(), expected_bytes.as_slice());
                let decoded = Vec::<Vec<Vec<$ty>>>::decode(&mut actual_bytes.as_slice())
                    .expect("Can't decode the vector");
                assert_eq!(decoded.len(), vector.len());
                assert_eq!(decoded.as_slice(), vector.as_slice());

                // Pop last byte to cause an error during decoding
                actual_bytes.pop();
                assert_eq!(
                    Vec::<Vec<Vec<$ty>>>::decode(&mut actual_bytes.as_slice()),
                    Err(Error::BufferIsTooShort)
                );
            }};
        }

        encode_decode_recursion!(Address, 10, 0);
        encode_decode_recursion!(AssetId, 10, 0);
        encode_decode_recursion!(ContractId, 10, 0);
        encode_decode_recursion!(Bytes4, 10, 4);
        encode_decode_recursion!(Bytes8, 10, 0);
        encode_decode_recursion!(Bytes20, 10, 4);
        encode_decode_recursion!(Bytes32, 10, 0);
        encode_decode_recursion!(MessageId, 10, 0);
        encode_decode_recursion!(Salt, 10, 0);

        encode_decode_recursion!(u16, 10, 6);
        encode_decode_recursion!(u32, 10, 4);
        encode_decode_recursion!(u64, 10, 0);
        encode_decode_recursion!(usize, 10, 0);
        encode_decode_recursion!(u128, 10, 0);

        encode_decode_recursion!(u8, 8, 0);
        encode_decode_recursion!(u8, 16, 0);
    }

    #[test]
    fn array_encode_decode_bytes() {
        macro_rules! encode_decode_bytes {
            ($num:expr, $padding:expr) => {{
                const NUM: usize = $num;
                const PADDING: usize = $padding;
                let rng = &mut StdRng::seed_from_u64(8586);
                let mut bytes: [u8; NUM] = [0u8; NUM];
                const PADDED_NUM: usize = NUM /* bytes */ + PADDING;
                for i in 0..NUM {
                    bytes[i] = rng.gen::<u8>();
                }
                assert_eq!(bytes.len(), NUM);

                // Correct sizes for each part
                assert_eq!(bytes.size_static(), PADDED_NUM);
                assert_eq!(bytes.size_dynamic(), 0);
                assert_eq!(bytes.size(), PADDED_NUM);

                // Correct encoding of static part
                let mut static_part = [0u8; PADDED_NUM];
                bytes
                    .encode_static(&mut static_part.as_mut())
                    .expect("Can't encode static part of bytes array");
                let expected_bytes = [bytes.as_slice(), [0u8; PADDING].as_slice()].concat();
                assert_eq!(static_part.len(), expected_bytes.len());
                assert_eq!(static_part.as_slice(), expected_bytes.as_slice());
                let decoded = <[u8; NUM] as Deserialize>::decode_static(&mut static_part.as_slice())
                    .expect("Can't decode static part of bytes array");
                assert_eq!(decoded.len(), bytes.len());
                assert_eq!(decoded.as_slice(), bytes.as_slice());

                // Empty encoding of dynamic part
                bytes
                    .encode_dynamic(&mut [].as_mut())
                    .expect("Can't encode dynamic part of bytes vector");

                // Correct encoding
                let mut actual_bytes = bytes.to_bytes();
                let expected_bytes = [bytes.as_slice(), [0u8; PADDING].as_slice()].concat();
                assert_eq!(actual_bytes.len(), expected_bytes.len());
                assert_eq!(actual_bytes.as_slice(), expected_bytes.as_slice());
                let decoded = <[u8; NUM] as Deserialize>::decode(&mut static_part.as_slice())
                    .expect("Can't decode bytes array");
                assert_eq!(decoded.len(), bytes.len());
                assert_eq!(decoded.as_slice(), bytes.as_slice());

                // Pop last byte to cause an error during decoding
                actual_bytes.pop();
                assert_eq!(
                    <[u8; NUM] as Deserialize>::decode(&mut actual_bytes.as_slice()),
                    Err(Error::BufferIsTooShort)
                );
            }};
        }

        encode_decode_bytes!(96, 0);
        encode_decode_bytes!(97, 7);
        encode_decode_bytes!(98, 6);
        encode_decode_bytes!(99, 5);
        encode_decode_bytes!(100, 4);
        encode_decode_bytes!(101, 3);
        encode_decode_bytes!(102, 2);
        encode_decode_bytes!(103, 1);
        encode_decode_bytes!(104, 0);

        assert_eq!(
            hex::encode(Serialize::to_bytes(&[0x11u8, 0x22u8, 0x33u8,])),
            "1122330000000000"
        );
        assert_eq!(
            hex::encode(Serialize::to_bytes(&[
                0x11u8, 0x22u8, 0x33u8, 0x44u8, 0x55u8, 0x66u8,
            ])),
            "1122334455660000"
        );
        assert_eq!(
            hex::encode(Serialize::to_bytes(&[
                0x11u8, 0x22u8, 0x33u8, 0x44u8, 0x55u8, 0x66u8, 0x77, 0x88,
            ])),
            "1122334455667788"
        );
    }

    #[test]
    fn array_encode_decode_not_bytes_with_recusrion() {
        macro_rules! encode_decode_not_bytes {
            ($ty:ty, $num:expr, $padding:expr) => {{
                const NUM: usize = $num;
                const PADDING: usize = $padding;
                let rng = &mut StdRng::seed_from_u64(8586);
                let mut array: [$ty; NUM] = [Default::default(); NUM];
                const PADDED_NUM: usize =
                    ::core::mem::size_of::<$ty>() * NUM + PADDING * NUM;
                for i in 0..NUM {
                    array[i] = rng.gen::<$ty>();
                }
                assert_eq!(array.len(), NUM);

                // Correct sizes for each part
                assert_eq!(array.size_static(), PADDED_NUM);
                assert_eq!(array.size_dynamic(), 0);
                assert_eq!(array.size(), PADDED_NUM);

                // Correct encoding of static part
                let mut static_part = [0u8; PADDED_NUM];
                array
                    .encode_static(&mut static_part.as_mut())
                    .expect("Can't encode static part of array");
                let expected_array = array
                    .clone()
                    .into_iter()
                    .flat_map(|e| e.to_bytes().into_iter())
                    .collect_vec();
                assert_eq!(static_part.len(), expected_array.len());
                assert_eq!(static_part.as_slice(), expected_array.as_slice());
                let decoded = <[$ty; NUM] as Deserialize>::decode_static(
                    &mut static_part.as_slice(),
                )
                .expect("Can't decode static part of array");
                assert_eq!(decoded.len(), array.len());
                assert_eq!(decoded.as_slice(), array.as_slice());

                // Empty encoding of dynamic part
                array
                    .encode_dynamic(&mut [].as_mut())
                    .expect("Can't encode dynamic part of array");

                // Correct encoding
                let mut actual_array = array.to_bytes();
                let expected_array = array
                    .clone()
                    .into_iter()
                    .flat_map(|e| e.to_bytes().into_iter())
                    .collect_vec();
                assert_eq!(actual_array.len(), expected_array.len());
                assert_eq!(actual_array.as_slice(), expected_array.as_slice());
                let decoded =
                    <[$ty; NUM] as Deserialize>::decode(&mut static_part.as_slice())
                        .expect("Can't decode array");
                assert_eq!(decoded.len(), array.len());
                assert_eq!(decoded.as_slice(), array.as_slice());

                // Pop last byte to cause an error during decoding
                actual_array.pop();
                assert_eq!(
                    <[$ty; NUM] as Deserialize>::decode(&mut actual_array.as_slice()),
                    Err(Error::BufferIsTooShort)
                );
            }};
        }

        encode_decode_not_bytes!(Address, 10, 0);
        encode_decode_not_bytes!(AssetId, 10, 0);
        encode_decode_not_bytes!(ContractId, 10, 0);
        encode_decode_not_bytes!(Bytes4, 10, 4);
        encode_decode_not_bytes!(Bytes8, 10, 0);
        encode_decode_not_bytes!(Bytes20, 10, 4);
        encode_decode_not_bytes!(Bytes32, 10, 0);
        encode_decode_not_bytes!(MessageId, 10, 0);
        encode_decode_not_bytes!(Salt, 10, 0);

        encode_decode_not_bytes!(u16, 10, 6);
        encode_decode_not_bytes!(u32, 10, 4);
        encode_decode_not_bytes!(u64, 10, 0);
        encode_decode_not_bytes!(usize, 10, 0);
        encode_decode_not_bytes!(u128, 10, 0);

        // Recursion level 1
        encode_decode_not_bytes!([u8; 8], 10, 0);
        encode_decode_not_bytes!([u16; 10], 10, 60);
        encode_decode_not_bytes!([u32; 10], 10, 40);
        encode_decode_not_bytes!([u64; 10], 10, 0);
        encode_decode_not_bytes!([u128; 10], 10, 0);
        encode_decode_not_bytes!([AssetId; 10], 10, 0);

        // Recursion level 2
        encode_decode_not_bytes!([[u8; 8]; 8], 10, 0);
        encode_decode_not_bytes!([[u16; 10]; 10], 10, 600);
        encode_decode_not_bytes!([[u32; 10]; 10], 10, 400);
        encode_decode_not_bytes!([[u64; 10]; 10], 10, 0);
        encode_decode_not_bytes!([[u128; 10]; 10], 10, 0);
        encode_decode_not_bytes!([[AssetId; 10]; 10], 10, 0);

        assert_eq!(
            hex::encode(Serialize::to_bytes(&[
                Bytes4::new([0x11u8, 0x22u8, 0x33u8, 0x44u8]),
                Bytes4::zeroed(),
                Bytes4::new([0x11u8, 0x22u8, 0x33u8, 0x44u8])
            ])),
            "112233440000000000000000000000001122334400000000"
        );

        assert_eq!(
            hex::encode(Serialize::to_bytes(&[0xAAu16, 0xBBu16, 0xCCu16, 0xDDu16,])),
            "00aa00000000000000bb00000000000000cc00000000000000dd000000000000"
        );
    }
}
// TODO: Add tests for structs, enums

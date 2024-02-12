//! Compressed encoding and decoding of Fuel types.
//! 
//! This is an extension of the canonical encoding.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::fmt;

use core::mem::MaybeUninit;
pub use fuel_derive::{
    Deserialize,
    Serialize,
};

/// Allows serialize the type into the `Output`.
pub trait SerializeCompact {
    /// Size of the static part of the serialized object, in bytes.
    fn size_static(&self) -> usize;

    /// Size of the dynamic part, in bytes.
    fn size_dynamic(&self) -> usize;

    /// Total size of the serialized object, in bytes.
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


/// Allows deserialize the type from the `Input`.
pub trait DeserializeCompact: Sized {
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


macro_rules! impl_for_primitives {
    ($t:ident, $unpadded:literal) => {
        impl SerializeCompact for $t {
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

        impl DeserializeCompact for $t {
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
            aligned_size(self.iter().map(|e| e.size()).sum())
        }
    }

    #[inline(always)]
    // Encode only the size of the vector. Elements will be encoded in the
    // `encode_dynamic` method.
    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        if self.len() > VEC_DECODE_LIMIT {
            return Err(Error::AllocationLimit)
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
            return Err(Error::AllocationLimit)
        }
        Ok(Vec::with_capacity(cap))
    }

    fn decode_dynamic<I: Input + ?Sized>(&mut self, buffer: &mut I) -> Result<(), Error> {
        for _ in 0..self.capacity() {
            // Bytes - Vec<u8> it a separate case without unpadding for each element.
            // It should unpadded at the end if is not % ALIGN
            if T::UNALIGNED_BYTES {
                let byte = buffer.read_byte()?;
                // SAFETY: `UNALIGNED_BYTES` implemented set for `u8`.
                let _self =
                    unsafe { ::core::mem::transmute::<&mut Vec<T>, &mut Vec<u8>>(self) };
                _self.push(byte);
            } else {
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
        self.iter().map(|e| e.size_static()).sum()
    }

    #[inline(always)]
    fn size_dynamic(&self) -> usize {
        self.iter().map(|e| e.size_dynamic()).sum()
    }

    #[inline(always)]
    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        let bytes = unsafe { ::core::mem::transmute::<&[T; N], &[u8; N]>(self) };
        buffer.write(bytes.as_slice())?;
        Ok(())
    }

    fn encode_dynamic<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        for e in self.iter() {
            e.encode_dynamic(buffer)?;
        }

        Ok(())
    }
}

impl<const N: usize, T: DeserializeCompact> DeserializeCompact for [T; N] {
    fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
        let mut bytes: [u8; N] = [0; N];
        buffer.read(bytes.as_mut())?;
        let ref_typed: &[T; N] = unsafe { core::mem::transmute(&bytes) };
        let typed: [T; N] = unsafe { core::ptr::read(ref_typed) };
        Ok(typed)
    }

    fn decode_dynamic<I: Input + ?Sized>(&mut self, buffer: &mut I) -> Result<(), Error> {
        for e in self.iter_mut() {
            e.decode_dynamic(buffer)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate<T: SerializeCompact + DeserializeCompact + Eq + core::fmt::Debug>(t: T) {
        let bytes = t.to_bytes();
        let t2 = T::from_bytes(&bytes).expect("Roundtrip failed");
        assert_eq!(t, t2);
        assert_eq!(t.to_bytes(), t2.to_bytes());

        let mut vec = Vec::new();
        t.encode_static(&mut vec).expect("Encode failed");
        assert_eq!(vec.len(), t.size_static());
    }

    fn validate_enum<T: SerializeCompact + DeserializeCompact + Eq + fmt::Debug>(t: T) {
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
    fn test_compact_encode_decode() {
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
}

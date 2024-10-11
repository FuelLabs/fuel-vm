// serde-big-array doesn't allow documentation of its generated structure
#![allow(missing_docs)]
use core::{
    array::TryFromSliceError,
    borrow::{
        Borrow,
        BorrowMut,
    },
    convert::TryFrom,
    fmt,
    ops::{
        Deref,
        DerefMut,
    },
    str,
};

#[cfg(feature = "random")]
use rand::{
    distributions::{
        Distribution,
        Standard,
    },
    Rng,
};

#[cfg(all(feature = "alloc", feature = "typescript"))]
use alloc::format;

macro_rules! key {
    ($i:ident, $s:expr) => {
        /// FuelVM atomic array type.
        #[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, postcard_bindgen::PostcardBindings)]
        #[repr(transparent)]
        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        #[derive(
            fuel_types::canonical::Serialize, fuel_types::canonical::Deserialize,
        )]
        pub struct $i([u8; $s]);

        key_methods!($i, $s);

        #[cfg(feature = "random")]
        impl Distribution<$i> for Standard {
            fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> $i {
                $i(rng.gen())
            }
        }
    };
}

macro_rules! key_with_big_array {
    ($i:ident, $s:expr) => {
        /// FuelVM atomic type.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        #[derive(
            fuel_types::canonical::Serialize, fuel_types::canonical::Deserialize,
        )]
        pub struct $i([u8; $s]);

        key_methods!($i, $s);

        impl Default for $i {
            fn default() -> $i {
                $i([0u8; $s])
            }
        }

        #[cfg(feature = "random")]
        impl Distribution<$i> for Standard {
            fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> $i {
                let mut bytes = $i::default();

                rng.fill_bytes(bytes.as_mut());

                bytes
            }
        }
    };
}

macro_rules! key_methods {
    ($i:ident, $s:expr) => {
        impl $i {
            /// Memory length of the type
            pub const LEN: usize = $s;

            /// Bytes constructor.
            pub const fn new(bytes: [u8; $s]) -> Self {
                Self(bytes)
            }

            /// Zeroes bytes constructor.
            pub const fn zeroed() -> $i {
                $i([0; $s])
            }

            #[cfg(feature = "unsafe")]
            #[allow(unsafe_code)]
            /// Add a conversion from arbitrary slices into owned
            ///
            /// # Safety
            ///
            /// This function will not panic if the length of the slice is smaller than
            /// `Self::LEN`. Instead, it will cause undefined behavior and read random
            /// disowned bytes
            pub unsafe fn from_slice_unchecked(bytes: &[u8]) -> Self {
                $i($crate::bytes::from_slice_unchecked(bytes))
            }

            /// Copy-free reference cast
            /// # Safety
            /// Assumes the type is `repr[transparent]`.
            pub fn from_bytes_ref_checked(bytes: &[u8]) -> Option<&Self> {
                let bytes: &[u8; $s] = bytes.get(..$s)?.try_into().ok()?;
                Some(Self::from_bytes_ref(bytes))
            }

            /// Copy-free reference cast
            /// # Safety
            /// Assumes the type is `repr[transparent]`.
            pub fn from_bytes_ref(bytes: &[u8; $s]) -> &Self {
                // The interpreter will frequently make references to keys and values
                // using logically checked slices.
                //
                // This function will save unnecessary copy to owned slices for the
                // interpreter access
                #[allow(unsafe_code)]
                unsafe {
                    &*(bytes.as_ptr() as *const Self)
                }
            }

            /// The memory size of the type by the method.
            pub const fn size(&self) -> usize {
                Self::LEN
            }
        }

        #[cfg(feature = "typescript")]
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $i {
            /// Bytes constructor.
            ///
            /// # Panics
            ///
            /// This function will panic if the length of `buf` is smaller than
            /// `Self::LEN`.
            #[wasm_bindgen(js_name = from_bytes)]
            pub fn from_bytes_typescript(bytes: &[u8]) -> Self {
                Self(bytes.try_into().expect(
                    format!("The size of the arrays it not {} size", $s).as_str(),
                ))
            }

            /// Zeroes bytes constructor.
            #[wasm_bindgen(js_name = zeroed)]
            pub fn zeroed_typescript() -> $i {
                Self::zeroed()
            }

            /// The memory size of the type by the method.
            #[wasm_bindgen(js_name = size)]
            pub fn size_typescript(&self) -> usize {
                self.size()
            }
        }

        #[cfg(feature = "random")]
        impl rand::Fill for $i {
            fn try_fill<R: rand::Rng + ?Sized>(
                &mut self,
                rng: &mut R,
            ) -> Result<(), rand::Error> {
                rng.fill_bytes(self.as_mut());

                Ok(())
            }
        }

        impl Deref for $i {
            type Target = [u8; $s];

            fn deref(&self) -> &[u8; $s] {
                &self.0
            }
        }

        impl Borrow<[u8; $s]> for $i {
            fn borrow(&self) -> &[u8; $s] {
                &self.0
            }
        }

        impl BorrowMut<[u8; $s]> for $i {
            fn borrow_mut(&mut self) -> &mut [u8; $s] {
                &mut self.0
            }
        }

        impl DerefMut for $i {
            fn deref_mut(&mut self) -> &mut [u8; $s] {
                &mut self.0
            }
        }

        impl AsRef<[u8]> for $i {
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

        impl AsMut<[u8]> for $i {
            fn as_mut(&mut self) -> &mut [u8] {
                &mut self.0
            }
        }

        impl From<[u8; $s]> for $i {
            fn from(bytes: [u8; $s]) -> Self {
                Self(bytes)
            }
        }

        impl From<$i> for [u8; $s] {
            fn from(salt: $i) -> [u8; $s] {
                salt.0
            }
        }

        impl TryFrom<&[u8]> for $i {
            type Error = TryFromSliceError;

            fn try_from(bytes: &[u8]) -> Result<$i, TryFromSliceError> {
                <[u8; $s]>::try_from(bytes).map(|b| b.into())
            }
        }

        impl fmt::LowerHex for $i {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if f.alternate() {
                    write!(f, "0x")?
                }

                if let Some(w) = f
                    .width()
                    .and_then(|w| Self::LEN.saturating_mul(2).checked_div(w))
                {
                    self.0.chunks(w).try_for_each(|c| {
                        write!(f, "{:02x}", c.iter().fold(0u8, |acc, x| acc ^ x))
                    })
                } else {
                    self.0.iter().try_for_each(|b| write!(f, "{:02x}", &b))
                }
            }
        }

        impl fmt::UpperHex for $i {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if f.alternate() {
                    write!(f, "0x")?
                }

                if let Some(w) = f
                    .width()
                    .and_then(|w| Self::LEN.saturating_mul(2).checked_div(w))
                {
                    self.0.chunks(w).try_for_each(|c| {
                        write!(f, "{:02X}", c.iter().fold(0u8, |acc, x| acc ^ x))
                    })
                } else {
                    self.0.iter().try_for_each(|b| write!(f, "{:02X}", &b))
                }
            }
        }

        impl fmt::Debug for $i {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                <Self as fmt::LowerHex>::fmt(&self, f)
            }
        }

        impl fmt::Display for $i {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                <Self as fmt::LowerHex>::fmt(&self, f)
            }
        }

        impl str::FromStr for $i {
            type Err = &'static str;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                const ERR: &str = concat!("Invalid encoded byte in ", stringify!($i));
                let mut ret = $i::zeroed();
                let s = s.strip_prefix("0x").unwrap_or(s);
                hex::decode_to_slice(&s, &mut ret.0).map_err(|_| ERR)?;
                Ok(ret)
            }
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $i {
            #[inline(always)]
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use alloc::format;
                use serde::ser::SerializeTuple;
                if serializer.is_human_readable() {
                    serializer.serialize_str(&format!("{:x}", &self))
                } else {
                    // Fixed-size arrays are tuples in serde data model
                    let mut arr = serializer.serialize_tuple($s)?;
                    for elem in &self.0 {
                        arr.serialize_element(elem)?;
                    }
                    arr.end()
                }
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $i {
            #[inline(always)]
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                use serde::de::Error;
                if deserializer.is_human_readable() {
                    let s: alloc::string::String =
                        serde::Deserialize::deserialize(deserializer)?;
                    s.parse().map_err(D::Error::custom)
                } else {
                    deserializer.deserialize_tuple($s, ArrayVisitor).map(Self)
                }
            }
        }
    };
}

key!(Address, 32);
key!(AssetId, 32);
key!(BlobId, 32);
key!(ContractId, 32);
key!(TxId, 32);
key!(Bytes4, 4);
key!(Bytes8, 8);
key!(Bytes20, 20);
key!(Bytes32, 32);
key!(Nonce, 32);
key!(MessageId, 32);
key!(Salt, 32);

key_with_big_array!(Bytes64, 64);

impl ContractId {
    /// Seed for the calculation of the contract id from its code.
    ///
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/src/identifiers/contract-id.md>
    pub const SEED: [u8; 4] = 0x4655454C_u32.to_be_bytes();
}

impl AssetId {
    pub const BASE: AssetId = AssetId::zeroed();
}

impl From<u64> for Nonce {
    fn from(value: u64) -> Self {
        let mut default = [0u8; 32];
        default[..8].copy_from_slice(&value.to_be_bytes());
        default.into()
    }
}

/// A visitor for deserializing a fixed-size byte array.
#[cfg(feature = "serde")]
struct ArrayVisitor<const S: usize>;

#[cfg(feature = "serde")]
impl<'de, const S: usize> serde::de::Visitor<'de> for ArrayVisitor<S> {
    type Value = [u8; S];

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an array of {S} bytes")
    }

    #[inline(always)]
    fn visit_borrowed_bytes<E>(self, items: &'de [u8]) -> Result<Self::Value, E> {
        let mut result = [0u8; S];
        result.copy_from_slice(items);
        Ok(result)
    }

    #[cfg(feature = "alloc")]
    #[inline(always)]
    fn visit_byte_buf<E>(self, v: alloc::vec::Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_borrowed_bytes(v.as_slice())
    }

    #[inline(always)]
    fn visit_seq<A>(self, mut value: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut arr = [0u8; S];
        for (i, elem) in arr.iter_mut().enumerate() {
            *elem = value
                .next_element()?
                .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
        }
        Ok(arr)
    }
}

/// Roundtrip serde encode/decode tests
#[cfg(all(test, feature = "serde"))]
mod tests_serde {
    use rand::{
        rngs::StdRng,
        SeedableRng,
    };

    use super::*;

    /// serde_json uses human-readable serialization by default
    #[test]
    fn test_human_readable() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let original: Address = rng.gen();
        let serialized = serde_json::to_string(&original).expect("Serialization failed");
        assert_eq!(
            serialized,
            "\"7bbd8a4ea06e94461b959ab18d35802bbac3cf47e2bf29195f7db2ce41630cd7\""
        );
        let recreated: Address =
            serde_json::from_str(&serialized).expect("Deserialization failed");
        assert_eq!(original, recreated);
    }

    /// postcard uses non-human-readable serialization
    #[test]
    fn test_not_human_readable() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let original: Address = rng.gen();
        let serialized = postcard::to_stdvec(&original).expect("Serialization failed");
        let expected_vec = original.0.to_vec();
        assert_eq!(&serialized, &expected_vec);
        let recreated: Address =
            postcard::from_bytes(&serialized).expect("Deserialization failed");
        assert_eq!(original, recreated);
    }

    /// postcard uses non-human-readable serialization
    #[test]
    fn test_not_human_readable_incorrect_deser() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let original: Address = rng.gen();
        let mut serialized =
            postcard::to_stdvec(&original).expect("Serialization failed");
        serialized.pop();
        let res: Result<Address, _> = postcard::from_bytes(&serialized);
        res.expect_err("Deserialization should have failed");
    }

    /// bincode uses non-human-readable serialization
    #[test]
    fn test_bincode() {
        let rng = &mut StdRng::seed_from_u64(8586);
        let original: Address = rng.gen();
        let serialized = bincode::serialize(&original).expect("Serialization failed");
        let recreated: Address =
            bincode::deserialize(&serialized).expect("Deserialization failed");
        assert_eq!(original, recreated);
    }
}

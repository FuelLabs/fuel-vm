use core::array::TryFromSliceError;
use core::borrow::Borrow;
use core::borrow::BorrowMut;
use core::convert::TryFrom;
use core::ops::{Add, Deref, DerefMut, Sub};
use core::{fmt, str};

#[cfg(feature = "random")]
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

use crate::hex_val;

macro_rules! key {
    ($i:ident, $t:ty) => {
        #[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        /// FuelVM atomic numeric type.
        #[repr(transparent)]
        pub struct $i($t);

        key_methods!($i, $t);

        #[cfg(feature = "random")]
        impl Distribution<$i> for Standard {
            fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> $i {
                $i(rng.gen())
            }
        }
    };
}

macro_rules! key_methods {
    ($i:ident, $t:ty) => {
        impl $i {
            /// Number constructor.
            pub const fn new(number: $t) -> Self {
                Self(number)
            }

            /// Convert to array of big endian bytes.
            pub fn to_bytes(self) -> [u8; 4] {
                self.0.to_be_bytes()
            }

            /// Convert to usize.
            pub const fn to_usize(self) -> usize {
                self.0 as usize
            }

            /// Convert to usize.
            pub const fn as_usize(&self) -> usize {
                self.0 as usize
            }
        }

        const SIZE: usize = core::mem::size_of::<$t>();

        #[cfg(feature = "random")]
        impl rand::Fill for $i {
            fn try_fill<R: rand::Rng + ?Sized>(&mut self, rng: &mut R) -> Result<(), rand::Error> {
                let number = rng.gen();
                *self = $i(number);

                Ok(())
            }
        }

        impl Deref for $i {
            type Target = $t;

            fn deref(&self) -> &$t {
                &self.0
            }
        }

        impl Borrow<$t> for $i {
            fn borrow(&self) -> &$t {
                &self.0
            }
        }

        impl BorrowMut<$t> for $i {
            fn borrow_mut(&mut self) -> &mut $t {
                &mut self.0
            }
        }

        impl DerefMut for $i {
            fn deref_mut(&mut self) -> &mut $t {
                &mut self.0
            }
        }

        impl From<[u8; SIZE]> for $i {
            fn from(bytes: [u8; SIZE]) -> Self {
                Self(<$t>::from_be_bytes(bytes))
            }
        }

        impl From<$t> for $i {
            fn from(value: $t) -> Self {
                Self(value)
            }
        }

        impl From<$i> for [u8; SIZE] {
            fn from(salt: $i) -> [u8; SIZE] {
                salt.0.to_be_bytes()
            }
        }

        impl From<$i> for $t {
            fn from(salt: $i) -> $t {
                salt.0
            }
        }

        impl TryFrom<&[u8]> for $i {
            type Error = TryFromSliceError;

            fn try_from(bytes: &[u8]) -> Result<$i, TryFromSliceError> {
                <[u8; SIZE]>::try_from(bytes).map(|b| b.into())
            }
        }

        impl fmt::LowerHex for $i {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if f.alternate() {
                    write!(f, "0x")?
                }

                let bytes = self.0.to_be_bytes();
                match f.width() {
                    Some(w) if w > 0 => bytes
                        .chunks(2 * bytes.len() / w)
                        .try_for_each(|c| write!(f, "{:02x}", c.iter().fold(0u8, |acc, x| acc ^ x))),

                    _ => bytes.iter().try_for_each(|b| write!(f, "{:02x}", &b)),
                }
            }
        }

        impl fmt::UpperHex for $i {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if f.alternate() {
                    write!(f, "0x")?
                }

                let bytes = self.0.to_be_bytes();
                match f.width() {
                    Some(w) if w > 0 => bytes
                        .chunks(2 * bytes.len() / w)
                        .try_for_each(|c| write!(f, "{:02X}", c.iter().fold(0u8, |acc, x| acc ^ x))),

                    _ => bytes.iter().try_for_each(|b| write!(f, "{:02X}", &b)),
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
                const ERR: &str = "Invalid encoded byte";

                let alternate = s.starts_with("0x");

                let mut b = s.bytes();
                let mut ret = <[u8; SIZE]>::default();

                if alternate {
                    b.next();
                    b.next();
                }

                for r in ret.as_mut() {
                    let h = b.next().and_then(hex_val).ok_or(ERR)?;
                    let l = b.next().and_then(hex_val).ok_or(ERR)?;

                    *r = h << 4 | l;
                }

                Ok(ret.into())
            }
        }

        impl Add for $i {
            type Output = $i;

            #[inline(always)]
            fn add(self, rhs: $i) -> $i {
                $i(self.0.wrapping_add(rhs.0))
            }
        }

        impl Sub for $i {
            type Output = $i;

            #[inline(always)]
            fn sub(self, rhs: $i) -> $i {
                $i(self.0.wrapping_sub(rhs.0))
            }
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $i {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use alloc::format;
                serializer.serialize_str(&format!("{:x}", &self))
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $i {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                use serde::de::Error;

                #[derive(serde::Deserialize)]
                #[serde(untagged)]
                enum MaybeStringed {
                    String(String),
                    Direct($t),
                }

                Ok(Self(match MaybeStringed::deserialize(deserializer)? {
                    MaybeStringed::String(s) => s.parse::<$t>().map_err(Error::custom)?,
                    MaybeStringed::Direct(i) => i,
                }))
            }
        }
    };
}

key!(BlockHeight, u32);

#[cfg(test)]
mod tests {
    use super::BlockHeight;

    #[cfg(feature = "serde")]
    #[test]
    fn test_block_height_serde() {
        assert!(serde_json::from_str::<BlockHeight>("0").unwrap().0 == 0);
        assert!(serde_json::from_str::<BlockHeight>("\"0\"").unwrap().0 == 0);
        assert!(serde_json::from_str::<BlockHeight>("12345678").unwrap().0 == 12345678);
        assert!(serde_json::from_str::<BlockHeight>("\"12345678\"").unwrap().0 == 12345678);

        assert!(serde_json::from_str::<BlockHeight>("").is_err());
        assert!(serde_json::from_str::<BlockHeight>("incorrect").is_err());
        assert!(serde_json::from_str::<BlockHeight>("0x12345678").is_err());
        assert!(serde_json::from_str::<BlockHeight>("12345678901234567890").is_err());
        assert!(serde_json::from_str::<BlockHeight>("\"incorrect\"").is_err());
        assert!(serde_json::from_str::<BlockHeight>("\"0x0\"").is_err());
        assert!(serde_json::from_str::<BlockHeight>("\"0x\"0\"").is_err());
    }
}

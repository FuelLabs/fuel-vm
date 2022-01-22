use crate::bytes;

use core::array::TryFromSliceError;
use core::convert::TryFrom;
use core::ops::{Deref, DerefMut};
use core::{fmt, str};

#[cfg(feature = "random")]
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

const fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'F' => Some(c - b'A' + 10),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'0'..=b'9' => Some(c - b'0'),
        _ => None,
    }
}

macro_rules! key {
    ($i:ident, $s:expr) => {
        #[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(
            feature = "serde-types-minimal",
            derive(serde::Serialize, serde::Deserialize)
        )]
        /// FuelVM atomic type.
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

macro_rules! key_no_default {
    ($i:ident, $s:expr) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        // TODO serde is not implemented for arrays bigger than 32 bytes
        /// FuelVM atomic type.
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

            /// Add a conversion from arbitrary slices into owned
            ///
            /// # Warning
            ///
            /// This function will not panic if the length of the slice is smaller than
            /// `Self::LEN`. Instead, it will cause undefined behavior and read random disowned
            /// bytes
            pub unsafe fn from_slice_unchecked(bytes: &[u8]) -> Self {
                $i(bytes::from_slice_unchecked(bytes))
            }

            /// Copy-free reference cast
            pub unsafe fn as_ref_unchecked(bytes: &[u8]) -> &Self {
                // The interpreter will frequently make references to keys and values using
                // logically checked slices.
                //
                // This function will save unnecessary copy to owned slices for the interpreter
                // access
                &*(bytes.as_ptr() as *const Self)
            }
        }

        #[cfg(feature = "random")]
        impl rand::Fill for $i {
            fn try_fill<R: rand::Rng + ?Sized>(&mut self, rng: &mut R) -> Result<(), rand::Error> {
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

                self.0.iter().try_for_each(|b| write!(f, "{:02x}", &b))
            }
        }

        impl fmt::UpperHex for $i {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if f.alternate() {
                    write!(f, "0x")?
                }

                self.0.iter().try_for_each(|b| write!(f, "{:02X}", &b))
            }
        }

        impl fmt::Debug for $i {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{:#x}", self)
            }
        }

        impl fmt::Display for $i {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{:#x}", self)
            }
        }

        impl str::FromStr for $i {
            type Err = &'static str;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                const ERR: &str = "Invalid encoded byte";

                let alternate = s.starts_with("0x");

                let mut b = s.bytes();
                let mut ret = $i::zeroed();

                if alternate {
                    b.next();
                    b.next();
                }

                for r in ret.as_mut() {
                    let h = b.next().and_then(hex_val).ok_or(ERR)?;
                    let l = b.next().and_then(hex_val).ok_or(ERR)?;

                    *r = h << 4 | l;
                }

                Ok(ret)
            }
        }
    };
}

key!(Address, 32);
key!(Color, 32);
key!(ContractId, 32);
key!(Bytes4, 4);
key!(Bytes8, 8);
key!(Bytes32, 32);
key!(Salt, 32);

key_no_default!(Bytes64, 64);

impl ContractId {
    /// Seed for the calculation of the contract id from its code.
    ///
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/identifiers.md#contract-id>
    pub const SEED: [u8; 4] = 0x4655454C_u32.to_be_bytes();
}

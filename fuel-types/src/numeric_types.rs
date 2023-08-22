use core::{
    array::TryFromSliceError,
    borrow::{
        Borrow,
        BorrowMut,
    },
    convert::TryFrom,
    fmt,
    ops::{
        Add,
        Deref,
        DerefMut,
        Sub,
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

use crate::hex_val;

macro_rules! key {
    ($i:ident, $t:ty) => {
        #[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        /// FuelVM atomic numeric type.
        #[repr(transparent)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        #[derive(
            fuel_types::canonical::Serialize, fuel_types::canonical::Deserialize,
        )]
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
        const _: () = {
            const SIZE: usize = core::mem::size_of::<$t>();

            impl $i {
                /// Number constructor.
                pub const fn new(number: $t) -> Self {
                    Self(number)
                }

                /// Convert to array of big endian bytes.
                pub fn to_bytes(self) -> [u8; SIZE] {
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

            #[cfg(feature = "typescript")]
            #[wasm_bindgen::prelude::wasm_bindgen]
            impl $i {
                #[wasm_bindgen::prelude::wasm_bindgen(constructor)]
                /// Number constructor.
                pub fn from_number(number: $t) -> Self {
                    Self(number)
                }

                /// Convert to array of big endian bytes.
                #[wasm_bindgen(js_name = to_bytes)]
                pub fn to_bytes_typescript(self) -> Vec<u8> {
                    self.to_bytes().to_vec()
                }

                /// Convert to usize.
                #[wasm_bindgen(js_name = as_usize)]
                pub fn as_usize_typescript(&self) -> usize {
                    self.0 as usize
                }
            }

            #[cfg(feature = "random")]
            impl rand::Fill for $i {
                fn try_fill<R: rand::Rng + ?Sized>(
                    &mut self,
                    rng: &mut R,
                ) -> Result<(), rand::Error> {
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
                        Some(w) if w > 0 => {
                            bytes.chunks(2 * bytes.len() / w).try_for_each(|c| {
                                write!(f, "{:02x}", c.iter().fold(0u8, |acc, x| acc ^ x))
                            })
                        }

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
                        Some(w) if w > 0 => {
                            bytes.chunks(2 * bytes.len() / w).try_for_each(|c| {
                                write!(f, "{:02X}", c.iter().fold(0u8, |acc, x| acc ^ x))
                            })
                        }

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
        };
    };
}

key!(BlockHeight, u32);
key!(ChainId, u64);

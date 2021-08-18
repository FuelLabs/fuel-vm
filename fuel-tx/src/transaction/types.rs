use crate::bytes;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use std::array::TryFromSliceError;
use std::convert::TryFrom;
use std::ops::{Deref, DerefMut};

mod input;
mod output;
mod witness;

macro_rules! key {
    ($i:ident, $s:expr) => {
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
        pub struct $i([u8; $s]);

        impl $i {
            pub const fn new(bytes: [u8; $s]) -> Self {
                Self(bytes)
            }

            pub const fn size_of() -> usize {
                $s
            }

            /// Add a conversion from arbitrary slices into owned
            ///
            /// # Warning
            ///
            /// This function will not panic if the length of the slice is smaller than
            /// `Self::size_of`. Instead, it will cause undefined behavior and read random disowned
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

        impl rand::Fill for $i {
            fn try_fill<R: rand::Rng + ?Sized>(&mut self, rng: &mut R) -> Result<(), rand::Error> {
                rng.fill_bytes(self.as_mut());

                Ok(())
            }
        }

        impl Distribution<$i> for Standard {
            fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> $i {
                $i(rng.gen())
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
    };
}

pub use input::Input;
pub use output::Output;
pub use witness::Witness;

key!(Address, 32);
key!(Color, 32);
key!(ContractId, 32);
key!(Bytes4, 4);
key!(Bytes8, 8);
key!(Bytes32, 32);
key!(Salt, 32);

impl ContractId {
    pub const SEED: [u8; 4] = 0x4655454C_u32.to_be_bytes();
}

#[cfg(test)]
mod tests {
    use crate::*;
    use rand::rngs::StdRng;
    use rand::{Rng, RngCore, SeedableRng};
    use std::convert::TryFrom;

    macro_rules! check_consistency {
        ($i:ident,$r:expr,$b:expr) => {
            unsafe {
                let n = $i::size_of();
                let s = $r.gen_range(0..$b.len() - n);
                let e = $r.gen_range(s + n..$b.len());
                let r = $r.gen_range(1..n - 1);
                let i = &$b[s..s + n];

                let a = $i::from_slice_unchecked(i);
                let b = $i::from_slice_unchecked(&$b[s..e]);
                let c = $i::try_from(i).expect("Memory conversion");

                // `d` will create random smaller slices and expect the value to be parsed correctly
                //
                // However, this is not the expected usage of the function
                let d = $i::from_slice_unchecked(&i[..i.len() - r]);

                let e = $i::as_ref_unchecked(i);

                // Assert `from_slice_unchecked` will not create two references to the same owned
                // memory
                assert_ne!(a.as_ptr(), b.as_ptr());

                // Assert `as_ref_unchecked` is copy-free
                assert_ne!(e.as_ptr(), a.as_ptr());
                assert_eq!(e.as_ptr(), i.as_ptr());

                assert_eq!(a, b);
                assert_eq!(a, c);
                assert_eq!(a, d);
                assert_eq!(&a, e);
            }
        };
    }

    #[test]
    fn from_slice_unchecked_safety() {
        let rng = &mut StdRng::seed_from_u64(8586);

        let mut bytes = [0u8; 257];
        rng.fill_bytes(&mut bytes);

        for _ in 0..100 {
            check_consistency!(Address, rng, bytes);
            check_consistency!(Color, rng, bytes);
            check_consistency!(ContractId, rng, bytes);
            check_consistency!(Bytes4, rng, bytes);
            check_consistency!(Bytes8, rng, bytes);
            check_consistency!(Bytes32, rng, bytes);
            check_consistency!(Salt, rng, bytes);
        }
    }
}

use crate::{OpcodeRepr, PanicReason};

macro_rules! from_u8 {
    ($i:ident) => {
        impl $i {
            /// Const fn implementation of `From<u8>`
            pub const fn from_u8(b: u8) -> Self {
                // Currently, the language doesn't support customized type coercion
                //
                // Safety: all possible values of `b` are either allocated or reserved
                unsafe { core::mem::transmute::<u8, Self>(b) }
            }
        }

        impl From<u8> for $i {
            fn from(b: u8) -> Self {
                Self::from_u8(b)
            }
        }

        impl From<$i> for u8 {
            fn from(i: $i) -> u8 {
                i as u8
            }
        }

        impl From<fuel_types::Word> for $i {
            fn from(w: fuel_types::Word) -> Self {
                Self::from_u8(w as u8)
            }
        }

        impl From<$i> for fuel_types::Word {
            fn from(i: $i) -> fuel_types::Word {
                (i as u8) as fuel_types::Word
            }
        }
    };
}

from_u8!(OpcodeRepr);
from_u8!(PanicReason);

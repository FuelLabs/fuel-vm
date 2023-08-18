//! Atomic types of the FuelVM.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(unsafe_code)]
#![warn(missing_docs)]
#![deny(unused_crate_dependencies)]

#[cfg(feature = "alloc")]
extern crate alloc;
extern crate core;

mod array_types;
#[cfg(feature = "alloc")]
mod fmt;
mod layout;
mod numeric_types;

pub use array_types::*;
#[cfg(feature = "alloc")]
pub use fmt::*;
pub use layout::*;
pub use numeric_types::*;

pub mod error;

pub use error::*;

/// Word-aligned bytes serialization functions.
pub mod bytes;

#[cfg(test)]
mod tests;

/// Register ID type
pub type RegisterId = usize;

/// Register value type
pub type Word = u64;

/// 6-bits immediate value type
pub type Immediate06 = u8;

/// 12-bits immediate value type
pub type Immediate12 = u16;

/// 18-bits immediate value type
pub type Immediate18 = u32;

/// 24-bits immediate value type
pub type Immediate24 = u32;

pub(crate) fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'F' => c.checked_sub(b'A').and_then(|c| c.checked_add(10)),
        b'a'..=b'f' => c.checked_sub(b'a').and_then(|c| c.checked_add(10)),
        b'0'..=b'9' => c.checked_sub(b'0'),
        _ => None,
    }
}

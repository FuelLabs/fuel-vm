//! Atomic types of the FuelVM.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(unsafe_code)]
#![warn(missing_docs)]
// #![deny(unused_crate_dependencies)]

// `fuel-derive` requires `fuel_types` import
extern crate self as fuel_types;

#[cfg(feature = "alloc")]
extern crate alloc;
extern crate core;
#[cfg(any(feature = "alloc", feature = "std"))]
pub mod canonical;

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

pub(crate) const fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'F' => Some(c - b'A' + 10),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'0'..=b'9' => Some(c - b'0'),
        _ => None,
    }
}

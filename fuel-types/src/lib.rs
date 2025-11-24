//! Atomic types of the FuelVM.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(unsafe_code)]
#![warn(missing_docs)]
#![deny(unused_crate_dependencies)]
#![deny(
    clippy::arithmetic_side_effects,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::string_slice
)]
// `fuel-derive` requires `fuel_types` import
// TODO: Move canonical serialization to `fuel-canonical` crate
#![allow(unused_crate_dependencies)]
extern crate self as fuel_types;

#[cfg(feature = "alloc")]
extern crate alloc;
extern crate core;

pub mod canonical;

mod array_types;
#[cfg(feature = "alloc")]
mod fmt;
mod numeric_types;

pub use array_types::*;
#[cfg(feature = "alloc")]
pub use fmt::*;
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

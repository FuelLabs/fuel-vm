//! Atomic types of the FuelVM.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod types;

pub use types::*;

/// Word-aligned bytes serialization functions.
pub mod bytes;

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

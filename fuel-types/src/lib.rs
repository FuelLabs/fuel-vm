#![cfg_attr(not(feature = "std"), no_std)]

pub mod bytes;

mod types;

pub use types::*;

#[cfg(feature = "std")]
mod data;

#[cfg(feature = "std")]
pub use data::*;

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

//! FuelVM opcodes representation

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

mod opcode;
mod types;

pub use opcode::Opcode;
pub use types::{Immediate06, Immediate12, Immediate18, Immediate24, RegisterId, Word};

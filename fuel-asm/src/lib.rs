//! FuelVM opcodes representation

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "std", doc = include_str!("../README.md"))]
#![warn(missing_docs)]

mod instruction;
mod opcode;
mod panic_reason;

mod macros;

#[doc(no_inline)]
pub use fuel_types::{self, Immediate06, Immediate12, Immediate18, Immediate24, RegisterId, Word};
pub use instruction::Instruction;
pub use opcode::{GMArgs, GTFArgs, Opcode, OpcodeRepr};
pub use panic_reason::{InstructionResult, PanicReason};

#[cfg(feature = "serde")]
#[doc(no_inline)]
pub use serde;

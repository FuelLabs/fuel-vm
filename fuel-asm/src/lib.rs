#![feature(arbitrary_enum_discriminant)]
#![feature(external_doc)]
#![warn(missing_docs)]
#![doc(include = "../README.md")]

mod opcode;
mod types;

pub use opcode::Opcode;
pub use types::{Immediate06, Immediate12, Immediate18, Immediate24, RegisterId, Word};

#![allow(clippy::try_err)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![allow(clippy::wrong_self_convention)]

pub mod call;
pub mod consts;
pub mod context;
pub mod contract;
pub mod crypto;
pub mod data;
pub mod error;
pub mod gas;
pub mod interpreter;
pub mod state;

pub mod prelude {
    pub use fuel_asm::Opcode;
    pub use fuel_storage::{MerkleRoot, MerkleStorage, Storage};
    pub use fuel_tx::{Input, Output, Receipt, Transaction, ValidationError, Witness};
    pub use fuel_types::{
        bytes::{Deserializable, SerializableVec, SizedBytes},
        Address, Bytes32, Bytes4, Bytes64, Bytes8, Color, ContractId, Immediate06, Immediate12, Immediate18,
        Immediate24, RegisterId, Salt, Word,
    };

    pub use crate::call::{Call, CallFrame};
    pub use crate::context::Context;
    pub use crate::contract::Contract;
    pub use crate::data::{InterpreterStorage, MemoryStorage};
    pub use crate::error::InterpreterError;
    pub use crate::interpreter::{Interpreter, InterpreterMetadata, MemoryRange};
    pub use crate::state::{Debugger, ProgramState, StateTransition, StateTransitionRef};

    #[cfg(feature = "debug")]
    pub use crate::state::{Breakpoint, DebugEval};
}

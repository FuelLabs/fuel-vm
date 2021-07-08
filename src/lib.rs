#![allow(clippy::try_err)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![allow(clippy::wrong_self_convention)]

pub mod consts;
pub mod crypto;
pub mod data;
pub mod debug;
pub mod interpreter;

pub mod prelude {
    pub use crate::data::{InterpreterStorage, MemoryStorage, Storage};
    pub use crate::debug::Debugger;
    pub use crate::interpreter::{
        Call, CallFrame, Context, Contract, ExecuteError, Interpreter, LogEvent, MemoryRange, ProgramState,
    };
    pub use fuel_asm::{Immediate06, Immediate12, Immediate18, Immediate24, Opcode, RegisterId, Word};
    pub use fuel_tx::{
        bytes::{Deserializable, SerializableVec, SizedBytes},
        Address, Bytes32, Color, ContractId, Input, Output, Salt, Transaction, ValidationError, Witness,
    };

    #[cfg(feature = "debug")]
    pub use crate::debug::{Breakpoint, DebugEval};
}

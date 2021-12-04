pub mod backtrace;
pub mod call;
pub mod client;
pub mod consts;
pub mod context;
pub mod contract;
pub mod crypto;
pub mod error;
pub mod gas;
pub mod interpreter;
pub mod state;
pub mod storage;
pub mod transactor;

pub mod prelude {
    //! Required implementations for full functionality

    pub use fuel_asm::{Instruction, InstructionResult, Opcode, OpcodeRepr, PanicReason};
    pub use fuel_storage::{MerkleRoot, MerkleStorage, Storage};
    pub use fuel_tx::{Input, Output, Receipt, Transaction, ValidationError, Witness};
    pub use fuel_types::{
        bytes::{Deserializable, SerializableVec, SizedBytes},
        Address, Bytes32, Bytes4, Bytes64, Bytes8, Color, ContractId, Immediate06, Immediate12, Immediate18,
        Immediate24, RegisterId, Salt, Word,
    };

    pub use crate::backtrace::Backtrace;
    pub use crate::call::{Call, CallFrame};
    pub use crate::client::{MemoryClient, MemoryStorage};
    pub use crate::context::Context;
    pub use crate::contract::Contract;
    pub use crate::error::InterpreterError;
    pub use crate::interpreter::{Interpreter, InterpreterMetadata, MemoryRange};
    pub use crate::state::{Debugger, ProgramState, StateTransition, StateTransitionRef};
    pub use crate::storage::InterpreterStorage;
    pub use crate::transactor::Transactor;

    #[cfg(feature = "debug")]
    pub use crate::state::{Breakpoint, DebugEval};
}

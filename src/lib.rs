//! FuelVM implementation

#![warn(missing_docs)]

pub mod backtrace;
pub mod call;
pub mod consts;
pub mod context;
pub mod crypto;
pub mod error;
pub mod gas;
pub mod interpreter;
pub mod memory_client;
pub mod state;
pub mod storage;
pub mod transactor;
pub mod util;

#[cfg(feature = "profile-any")]
pub mod profiler;

// Fully re-export fuel dependencies
#[doc(no_inline)]
pub use fuel_asm;
#[doc(no_inline)]
pub use fuel_crypto;
#[doc(no_inline)]
pub use fuel_merkle;
#[doc(no_inline)]
pub use fuel_storage;
#[doc(no_inline)]
pub use fuel_tx;
#[doc(no_inline)]
pub use fuel_types;

pub mod prelude {
    //! Required implementations for full functionality
    #[doc(no_inline)]
    pub use fuel_asm::{Instruction, InstructionResult, Opcode, OpcodeRepr, PanicReason};
    #[doc(no_inline)]
    pub use fuel_storage::{MerkleRoot, MerkleStorage, Storage};
    #[doc(no_inline)]
    pub use fuel_tx::{
        ConsensusParameters, Contract, Input, Output, Receipt, Transaction, UtxoId, ValidationError, Witness,
    };
    #[doc(no_inline)]
    pub use fuel_types::{
        bytes::{Deserializable, SerializableVec, SizedBytes},
        Address, AssetId, Bytes32, Bytes4, Bytes64, Bytes8, ContractId, Immediate06, Immediate12, Immediate18,
        Immediate24, RegisterId, Salt, Word,
    };

    pub use crate::backtrace::Backtrace;
    pub use crate::call::{Call, CallFrame};
    pub use crate::context::Context;
    pub use crate::error::{Infallible, InterpreterError, RuntimeError};
    pub use crate::interpreter::{Interpreter, InterpreterMetadata, MemoryRange};
    pub use crate::memory_client::MemoryClient;
    pub use crate::state::{Debugger, ProgramState, StateTransition, StateTransitionRef};
    pub use crate::storage::{InterpreterStorage, MemoryStorage, PredicateStorage};
    pub use crate::transactor::Transactor;

    #[cfg(feature = "debug")]
    pub use crate::state::{Breakpoint, DebugEval};
}

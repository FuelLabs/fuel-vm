//! FuelVM implementation

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![deny(unused_crate_dependencies)]

pub mod arith;
pub mod backtrace;
pub mod call;
pub mod checked_transaction;
pub mod constraints;
pub mod consts;
pub mod context;
pub mod crypto;
pub mod error;
pub mod gas;
pub mod interpreter;
pub mod memory_client;
pub mod predicate;
pub mod state;
pub mod storage;
pub mod transactor;
pub mod util;

#[cfg(feature = "profile-any")]
pub mod profiler;

#[cfg(test)]
mod tests;

#[cfg(not(feature = "profile-any"))]
/// Placeholder
pub mod profiler {
    use crate::constraints::InstructionLocation;

    /// Placeholder profiler.
    #[derive(Default, Debug, Clone)]
    pub struct Profiler;

    impl Profiler {
        /// Set the current coverage location.
        pub fn set_coverage(&mut self, _location: InstructionLocation) {}

        /// Add gas to the current coverage location.
        pub fn add_gas(&mut self, _location: InstructionLocation, _gas_use: u64) {}
    }
}

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
    pub use fuel_asm::{
        GMArgs,
        GTFArgs,
        Instruction,
        Opcode,
        PanicReason,
    };
    #[doc(no_inline)]
    pub use fuel_crypto::{
        Hasher,
        Message,
        PublicKey,
        SecretKey,
        Signature,
    };
    #[doc(no_inline)]
    pub use fuel_storage::{
        MerkleRoot,
        MerkleRootStorage,
        StorageAsMut,
        StorageAsRef,
        StorageInspect,
        StorageMutate,
    };
    #[doc(no_inline)]
    pub use fuel_tx::*;
    #[doc(no_inline)]
    pub use fuel_types::{
        bytes::{
            Deserializable,
            SerializableVec,
            SizedBytes,
        },
        Address,
        AssetId,
        Bytes32,
        Bytes4,
        Bytes64,
        Bytes8,
        ContractId,
        Immediate06,
        Immediate12,
        Immediate18,
        Immediate24,
        RegisterId,
        Salt,
        Word,
    };

    pub use crate::{
        backtrace::Backtrace,
        call::{
            Call,
            CallFrame,
        },
        context::Context,
        error::{
            Bug,
            BugId,
            BugVariant,
            Infallible,
            InterpreterError,
            RuntimeError,
        },
        gas::{
            GasCosts,
            GasCostsValues,
        },
        interpreter::{
            ExecutableTransaction,
            Interpreter,
            MemoryRange,
        },
        memory_client::MemoryClient,
        predicate::RuntimePredicate,
        state::{
            Debugger,
            ProgramState,
            StateTransition,
            StateTransitionRef,
        },
        storage::{
            InterpreterStorage,
            MemoryStorage,
            PredicateStorage,
        },
        transactor::Transactor,
    };

    #[cfg(feature = "debug")]
    pub use crate::state::{
        Breakpoint,
        DebugEval,
    };

    #[cfg(any(test, feature = "test-helpers"))]
    pub use crate::util::test_helpers::TestBuilder;

    #[cfg(any(test, feature = "test-helpers"))]
    pub use crate::checked_transaction::{
        builder::TransactionBuilderExt,
        IntoChecked,
    };

    #[cfg(all(feature = "profile-gas", any(test, feature = "test-helpers")))]
    pub use crate::util::gas_profiling::GasProfiler;

    pub use crate::profiler::Profiler;
    #[cfg(feature = "profile-any")]
    pub use crate::profiler::{
        CoverageProfilingData,
        GasProfilingData,
        InstructionLocation,
        PerLocationIter,
        PerLocationKeys,
        PerLocationValues,
        ProfileReceiver,
        ProfilingData,
        StderrReceiver,
    };
}

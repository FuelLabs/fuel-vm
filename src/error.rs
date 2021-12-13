//! Runtime interpreter error implementation

use fuel_asm::{Instruction, InstructionResult, PanicReason};
use fuel_tx::ValidationError;

use std::convert::Infallible;
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// Interpreter runtime error variants.
pub enum InterpreterError {
    /// The instructions execution resulted in a well-formed panic, caused by an
    /// explicit instruction.
    PanicInstruction(InstructionResult),
    /// The VM execution resulted in a well-formed panic. This panic wasn't
    /// caused by an instruction contained in the transaction or a called
    /// contract.
    Panic(PanicReason),
    /// The initalization routine panicked. This is an internal critical error
    /// and cannot be caused by inconsistent inputs/transactions.
    Initialization(PanicReason),
    /// The provided transaction isn't valid.
    ValidationError(ValidationError),
    /// The predicate verification failed.
    PredicateFailure,
    /// No transaction was initialized in the interpreter. It cannot provide
    /// state transitions.
    NoTransactionInitialized,

    #[cfg(feature = "debug")]
    /// The debug state is not initialized; debug routines can't be called.
    DebugStateNotInitialized,
}

impl InterpreterError {
    /// Return the specified panic reason that caused this error, if applicable.
    pub const fn panic_reason(&self) -> Option<PanicReason> {
        match self {
            Self::PanicInstruction(result) => Some(*result.reason()),
            Self::Panic(reason) | Self::Initialization(reason) => Some(*reason),
            _ => None,
        }
    }

    /// Return the instruction that caused this error, if applicable.
    pub const fn instruction(&self) -> Option<&Instruction> {
        match self {
            Self::PanicInstruction(result) => Some(result.instruction()),
            _ => None,
        }
    }

    /// Return the underlying `InstructionResult` if this instance is
    /// `PanicInstruction`; returns `None` otherwise.
    pub fn instruction_result(&self) -> Option<&InstructionResult> {
        match self {
            Self::PanicInstruction(r) => Some(r),
            _ => None,
        }
    }
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidationError(e) => {
                write!(f, "Failed to validate the transaction: {}", e)
            }

            _ => write!(f, "Execution error: {:?}", self),
        }
    }
}

impl StdError for InterpreterError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::ValidationError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ValidationError> for InterpreterError {
    fn from(e: ValidationError) -> Self {
        Self::ValidationError(e)
    }
}

impl From<Infallible> for InterpreterError {
    fn from(_i: Infallible) -> InterpreterError {
        unreachable!()
    }
}

impl From<InstructionResult> for InterpreterError {
    fn from(r: InstructionResult) -> InterpreterError {
        Self::PanicInstruction(r)
    }
}

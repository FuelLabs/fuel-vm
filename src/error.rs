//! Runtime interpreter error implementation

use fuel_asm::{Instruction, InstructionResult, PanicReason};
use fuel_tx::ValidationError;

use std::convert::Infallible;
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InterpreterError {
    PanicInstruction(InstructionResult),
    Panic(PanicReason),
    Initialization(PanicReason),
    ValidationError(ValidationError),
    PredicateFailure,
    NoTransactionInitialized,

    #[cfg(feature = "debug")]
    DebugStateNotInitialized,
}

impl InterpreterError {
    pub const fn panic_reason(&self) -> Option<PanicReason> {
        match self {
            Self::PanicInstruction(result) => Some(*result.reason()),
            Self::Panic(reason) | Self::Initialization(reason) => Some(*reason),
            _ => None,
        }
    }

    pub const fn instruction(&self) -> Option<&Instruction> {
        match self {
            Self::PanicInstruction(result) => Some(result.instruction()),
            _ => None,
        }
    }

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

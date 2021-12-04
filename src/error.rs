use fuel_asm::{Instruction, InstructionResult, PanicReason};
use fuel_tx::ValidationError;

use std::convert::Infallible;
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InterpreterError {
    PanicInstruction(PanicReason, Instruction),
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
            Self::PanicInstruction(reason, _) | Self::Panic(reason) | Self::Initialization(reason) => Some(*reason),
            _ => None,
        }
    }

    pub const fn instruction(&self) -> Option<&Instruction> {
        match self {
            Self::PanicInstruction(_, instruction) => Some(instruction),
            _ => None,
        }
    }

    /// Attempt to generate an instruction result variant, depending on the
    /// class of error.
    ///
    /// This instruction result represents runtime errors that are the product
    /// of wrong programs and should be predicted and well-defined in the
    /// specs.
    pub fn instruction_result(&self) -> Option<InstructionResult> {
        self.panic_reason()
            .zip(self.instruction().copied())
            .map(|(r, i)| InstructionResult::error(r, i))
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

impl From<PanicReason> for InterpreterError {
    fn from(_r: PanicReason) -> Self {
        unimplemented!()
    }
}

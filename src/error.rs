use fuel_asm::Opcode;
use fuel_tx::ValidationError;

use std::convert::Infallible;
use std::error::Error as StdError;
use std::{fmt, io};

#[derive(Debug)]
pub enum InterpreterError {
    OpcodeFailure(Opcode),
    OpcodeUnimplemented(Opcode),
    ValidationError(ValidationError),
    Io(io::Error),
    TransactionCreateStaticContractNotFound,
    TransactionCreateIdNotInTx,
    ArithmeticOverflow,
    StackOverflow,
    PredicateOverflow,
    ProgramOverflow,
    PredicateFailure,
    ContractNotFound,
    MemoryOverflow,
    MemoryOwnership,
    ContractNotInTxInputs,
    NotEnoughBalance,
    ExpectedInternalContext,
    ExternalColorNotFound,
    OutOfGas,
    InputNotFound,
    OutputNotFound,
    WitnessNotFound,
    TxMaturity,
    MetadataIdentifierUndefined,

    #[cfg(feature = "debug")]
    DebugStateNotInitialized,
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpcodeFailure(op) => {
                write!(f, "Failed to execute the opcode: {:?}", op)
            }

            Self::ValidationError(e) => {
                write!(f, "Failed to validate the transaction: {}", e)
            }

            Self::Io(e) => {
                write!(f, "I/O failure: {}", e)
            }

            _ => write!(f, "Execution error: {:?}", self),
        }
    }
}

impl StdError for InterpreterError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::ValidationError(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ValidationError> for InterpreterError {
    fn from(e: ValidationError) -> Self {
        Self::ValidationError(e)
    }
}

impl From<io::Error> for InterpreterError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<Infallible> for InterpreterError {
    fn from(_i: Infallible) -> InterpreterError {
        unreachable!()
    }
}

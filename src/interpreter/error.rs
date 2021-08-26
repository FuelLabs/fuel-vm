use crate::data::DataError;

use fuel_asm::Opcode;
use fuel_tx::ValidationError;

use std::{error, fmt, io};

#[derive(Debug)]
pub enum ExecuteError {
    OpcodeFailure(Opcode),
    OpcodeUnimplemented(Opcode),
    ValidationError(ValidationError),
    Io(io::Error),
    Data(DataError),
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

    #[cfg(feature = "debug")]
    DebugStateNotInitialized,
}

impl fmt::Display for ExecuteError {
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

impl error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::ValidationError(e) => Some(e),
            Self::Io(e) => Some(e),
            Self::Data(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ValidationError> for ExecuteError {
    fn from(e: ValidationError) -> Self {
        Self::ValidationError(e)
    }
}

impl From<io::Error> for ExecuteError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<DataError> for ExecuteError {
    fn from(e: DataError) -> Self {
        Self::Data(e)
    }
}

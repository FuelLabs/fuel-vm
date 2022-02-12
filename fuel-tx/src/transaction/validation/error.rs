use core::fmt;

#[cfg(feature = "std")]
use std::{error, io};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde-types-minimal",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum ValidationError {
    InputCoinPredicateLength { index: usize },
    InputCoinPredicateDataLength { index: usize },
    InputCoinWitnessIndexBounds { index: usize },
    InputCoinInvalidSignature { index: usize },
    InputContractAssociatedOutputContract { index: usize },
    OutputContractInputIndex { index: usize },
    TransactionCreateInputContract { index: usize },
    TransactionCreateOutputContract { index: usize },
    TransactionCreateOutputVariable { index: usize },
    TransactionCreateOutputChangeNotBaseAsset { index: usize },
    TransactionCreateOutputContractCreatedMultiple { index: usize },
    TransactionCreateBytecodeLen,
    TransactionCreateBytecodeWitnessIndex,
    TransactionCreateStaticContractsMax,
    TransactionCreateStaticContractsOrder,
    TransactionCreateStorageSlotMax,
    TransactionCreateStorageSlotOrder,
    TransactionScriptLength,
    TransactionScriptDataLength,
    TransactionScriptOutputContractCreated { index: usize },
    TransactionGasLimit,
    TransactionMaturity,
    TransactionInputsMax,
    TransactionOutputsMax,
    TransactionWitnessesMax,
    TransactionOutputChangeColorDuplicated,
    TransactionOutputChangeColorNotFound,
    TransactionOutputVariableColorDuplicated,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO better describe the error variants
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl error::Error for ValidationError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[cfg(feature = "std")]
impl From<ValidationError> for io::Error {
    fn from(v: ValidationError) -> io::Error {
        io::Error::new(io::ErrorKind::Other, v)
    }
}

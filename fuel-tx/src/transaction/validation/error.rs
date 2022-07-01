use core::fmt;

use crate::UtxoId;
use fuel_types::{AssetId, ContractId, MessageId};
#[cfg(feature = "std")]
use std::{error, io};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ValidationError {
    InputWitnessIndexBounds {
        index: usize,
    },
    InputPredicateEmpty {
        index: usize,
    },
    InputPredicateLength {
        index: usize,
    },
    InputPredicateDataLength {
        index: usize,
    },
    InputPredicateOwner {
        index: usize,
    },
    InputInvalidSignature {
        index: usize,
    },
    InputContractAssociatedOutputContract {
        index: usize,
    },
    InputMessageDataLength {
        index: usize,
    },
    DuplicateInputUtxoId {
        utxo_id: UtxoId,
    },
    DuplicateMessageInputId {
        message_id: MessageId,
    },
    DuplicateInputContractId {
        contract_id: ContractId,
    },
    OutputContractInputIndex {
        index: usize,
    },
    TransactionCreateInputContract {
        index: usize,
    },
    TransactionCreateOutputContract {
        index: usize,
    },
    TransactionCreateOutputVariable {
        index: usize,
    },
    TransactionCreateOutputChangeNotBaseAsset {
        index: usize,
    },
    TransactionCreateOutputContractCreatedMultiple {
        index: usize,
    },
    TransactionCreateBytecodeLen,
    TransactionCreateBytecodeWitnessIndex,
    TransactionCreateStorageSlotMax,
    TransactionCreateStorageSlotOrder,
    TransactionScriptLength,
    TransactionScriptDataLength,
    TransactionScriptOutputContractCreated {
        index: usize,
    },
    TransactionGasLimit,
    TransactionMaturity,
    TransactionInputsMax,
    TransactionOutputsMax,
    TransactionWitnessesMax,
    TransactionOutputChangeAssetIdDuplicated,
    TransactionOutputChangeAssetIdNotFound(AssetId),
    /// This error happens when a transaction attempts to create a coin output for an asset type
    /// that doesn't exist in the coin inputs.
    TransactionOutputCoinAssetIdNotFound(AssetId),
    /// The transaction doesn't provide enough input amount of the native chain asset to cover
    /// all potential execution fees
    InsufficientFeeAmount {
        /// The expected amount of fees required to cover the transaction
        expected: u64,
        /// The fee amount actually provided for spending
        provided: u64,
    },
    /// The transaction doesn't provide enough input amount of the given asset to cover the
    /// amounts used in the outputs.
    InsufficientInputAmount {
        /// The asset id being spent
        asset: AssetId,
        /// The amount expected by a coin output
        expected: u64,
        /// The total amount provided by coin inputs
        provided: u64,
    },
    /// The user provided amounts for coins or gas prices that caused an arithmetic
    /// overflow.
    ArithmeticOverflow,
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

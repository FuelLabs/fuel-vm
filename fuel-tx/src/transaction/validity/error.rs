use crate::UtxoId;
use fuel_types::{
    AssetId,
    ContractId,
    MessageId,
};

/// The error returned during the checking of the transaction's validity rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ValidityError {
    /// Transaction doesn't have spendable input message or coin.
    NoSpendableInput,
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
    /// The `Create` transaction contains (retryable) message input.
    TransactionCreateMessageData {
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
    TransactionCreateOutputContractCreatedDoesntMatch {
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
    /// The block height of the checking doesn't match the transaction's block height.
    /// `Mint` transaction only exists in the scope of the block.
    TransactionMintIncorrectBlockHeight,
    /// The `Output.input_index` is not zero.
    TransactionMintIncorrectOutputIndex,
    /// The `Output.mint_base_asset` is not base asset.
    TransactionMintNonBaseAsset,
    /// The transaction exceeded the size limit.
    TransactionSizeLimitExceeded,
    /// Max gas per tx exceeded
    TransactionMaxGasExceeded,
    TransactionMaxFeeLimitExceeded,
    TransactionWitnessLimitExceeded,
    TransactionPoliciesAreInvalid,
    TransactionNoGasPricePolicy,
    TransactionMaturity,
    TransactionInputsMax,
    TransactionOutputsMax,
    TransactionWitnessesMax,
    TransactionOutputChangeAssetIdDuplicated(AssetId),
    TransactionOutputChangeAssetIdNotFound(AssetId),
    /// This error happens when a transaction attempts to create a coin output for an
    /// asset type that doesn't exist in the coin inputs.
    TransactionOutputCoinAssetIdNotFound(AssetId),
    /// The transaction doesn't provide enough input amount of the native chain asset to
    /// cover all potential execution fees
    #[display(
        fmt = "Insufficient fee amount: expected {}, provided {}",
        expected,
        provided
    )]
    InsufficientFeeAmount {
        /// The expected amount of fees required to cover the transaction
        expected: u64,
        /// The fee amount actually provided for spending
        provided: u64,
    },
    /// The transaction doesn't provide enough input amount of the given asset to cover
    /// the amounts used in the outputs.
    #[display(
        fmt = "Insufficient input amount: asset {}, expected {}, provided {}",
        asset,
        expected,
        provided
    )]
    InsufficientInputAmount {
        /// The asset id being spent
        asset: AssetId,
        /// The amount expected by a coin output
        expected: u64,
        /// The total amount provided by coin inputs
        provided: u64,
    },
    /// The given coins is too large
    BalanceOverflow,
    /// The given gas costs is are too large
    GasCostsCoinsOverflow,
}

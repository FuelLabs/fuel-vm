use crate::UtxoId;
use fuel_types::{
    AssetId,
    ContractId,
    Nonce,
};

/// The error returned during the checking of the transaction's validity rules.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
#[non_exhaustive]
pub enum ValidityError {
    /// The actual and calculated metadata of the transaction mismatch.
    TransactionMetadataMismatch,
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
    DuplicateInputNonce {
        nonce: Nonce,
    },
    DuplicateInputContractId {
        contract_id: ContractId,
    },
    OutputContractInputIndex {
        index: usize,
    },
    /// One of inputs' `AssetId` is not base asset id.
    TransactionInputContainsNonBaseAssetId {
        index: usize,
    },
    /// One of inputs is a `Input::Contract` when it is not allowed.
    TransactionInputContainsContract {
        index: usize,
    },
    /// One of inputs contains retryable message when it is not allowed.
    TransactionInputContainsMessageData {
        index: usize,
    },
    /// One of outputs is a `Output::Contract` when it is not allowed.
    TransactionOutputContainsContract {
        index: usize,
    },
    /// One of outputs is a `Output::Variable` when it is not allowed.
    TransactionOutputContainsVariable {
        index: usize,
    },
    /// One of `Output::Change` outputs uses a non-base asset id.
    TransactionChangeChangeUsesNotBaseAsset {
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
    /// The output contains a `Output::ContractCreated` which is not allowed.
    TransactionOutputContainsContractCreated {
        index: usize,
    },
    /// The block height of the checking doesn't match the transaction's block height.
    /// `Mint` transaction only exists in the scope of the block.
    TransactionMintIncorrectBlockHeight,
    /// The `Output.input_index` is not zero.
    TransactionMintIncorrectOutputIndex,
    /// The `Output.mint_base_asset` is not base asset.
    TransactionMintNonBaseAsset,
    /// The `Upgrade` transaction doesn't have the privileged address as the input
    /// owner.
    TransactionUpgradeNoPrivilegedAddress,
    /// The `Upgrade` transaction's checksum doesn't match the consensus parameters from
    /// witness.
    TransactionUpgradeConsensusParametersChecksumMismatch,
    /// The `Upgrade` transaction's consensus parameters serialization failed.
    TransactionUpgradeConsensusParametersSerialization,
    /// The `Upgrade` transaction's consensus parameters deserialization failed.
    TransactionUpgradeConsensusParametersDeserialization,
    /// The verification of the bytecode root of the `Upload` transaction failed.
    TransactionUploadRootVerificationFailed,
    /// The total number of bytecode subsections in the `Upload` transaction exceeds the
    /// limit.
    TransactionUploadTooManyBytecodeSubsections,
    /// The transaction exceeded the size limit.
    TransactionSizeLimitExceeded,
    /// Max gas per tx exceeded
    TransactionMaxGasExceeded,
    TransactionWitnessLimitExceeded,
    TransactionPoliciesAreInvalid,
    TransactionNoGasPricePolicy,
    TransactionMaturity,
    TransactionExpiration,
    TransactionMaxFeeNotSet,
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
        "Insufficient fee amount: expected {}, provided {}",
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
        "Insufficient input amount: asset {}, expected {}, provided {}",
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
    /// Serialized input length is too large.
    SerializedInputTooLarge {
        index: usize,
    },
    /// Serialized output length is too large.
    SerializedOutputTooLarge {
        index: usize,
    },
    /// Serialized witness length is too large.
    SerializedWitnessTooLarge {
        index: usize,
    },
    /// The `Create` transaction doesn't contain `Output::ContractCreated`.
    TransactionOutputDoesntContainContractCreated,
    /// Blob id of the transaction differs from the data.
    TransactionBlobIdVerificationFailed,
}

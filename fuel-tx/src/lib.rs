#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::try_err)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![allow(clippy::wrong_self_convention)]
#![deny(
    clippy::arithmetic_side_effects,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::string_slice
)]
#![deny(unused_crate_dependencies)]
#![deny(unsafe_code)]

// TODO: Add docs

#[cfg(feature = "alloc")]
extern crate alloc;
extern crate core;

pub mod consts;
mod tx_pointer;

pub use fuel_asm::{
    PanicInstruction,
    PanicReason,
};
pub use fuel_types::{
    Address,
    AssetId,
    BlobId,
    Bytes4,
    Bytes8,
    Bytes32,
    Bytes64,
    ContractId,
    MessageId,
    Salt,
    SubAssetId,
    Word,
};
pub use tx_pointer::TxPointer;

#[cfg(feature = "test-helpers")]
mod builder;

#[cfg(feature = "alloc")]
mod contract;

#[cfg(feature = "alloc")]
mod receipt;

#[cfg(feature = "alloc")]
mod transaction;

#[cfg(test)]
mod tests;

#[cfg(feature = "test-helpers")]
pub mod test_helper;

#[cfg(feature = "test-helpers")]
pub use builder::{
    Buildable,
    Finalizable,
    TransactionBuilder,
};

#[cfg(feature = "alloc")]
pub use receipt::{
    Receipt,
    ScriptExecutionResult,
};

#[cfg(feature = "alloc")]
pub use transaction::{
    Blob,
    BlobBody,
    BlobIdExt,
    BlobMetadata,
    Cacheable,
    Chargeable,
    ChargeableMetadata,
    ChargeableTransaction,
    ConsensusParameters,
    ContractParameters,
    Create,
    CreateMetadata,
    DependentCost,
    Executable,
    FeeParameters,
    FormatValidityChecks,
    GasCosts,
    GasCostsValues,
    Mint,
    PredicateParameters,
    Script,
    ScriptCode,
    ScriptParameters,
    StorageSlot,
    Transaction,
    TransactionFee,
    TransactionRepr,
    TxId,
    TxParameters,
    Upgrade,
    UpgradeBody,
    UpgradeMetadata,
    UpgradePurpose,
    Upload,
    UploadBody,
    UploadMetadata,
    UploadSubsection,
    UtxoId,
    ValidityError,
    Witness,
    consensus_parameters,
    field,
    input,
    input::Input,
    input::InputRepr,
    output,
    output::Output,
    output::OutputRepr,
    policies,
};

#[cfg(feature = "chargeable-tx-v2")]
pub use transaction::ScriptV2;

#[cfg(feature = "da-compression")]
pub use transaction::{
    BodyConstraints,
    CompressedMint,
    CompressedTransaction,
    CompressedUtxoId,
};

pub use transaction::{
    PrepareSign,
    Signable,
    UniqueIdentifier,
};

#[cfg(feature = "alloc")]
pub use contract::Contract;

/// Trait extends the functionality of the `ContractId` type.
pub trait ContractIdExt {
    /// Creates an `AssetId` from the `ContractId` and `sub_id`.
    fn asset_id(&self, sub_id: &SubAssetId) -> AssetId;

    /// Creates an `AssetId` from the `ContractId` and the default 0x00..000 `sub_id`.
    fn default_asset(&self) -> AssetId;
}

impl ContractIdExt for ContractId {
    fn asset_id(&self, sub_id: &SubAssetId) -> AssetId {
        let hasher = fuel_crypto::Hasher::default();
        AssetId::new(
            *hasher
                .chain(self.as_slice())
                .chain(sub_id.as_slice())
                .finalize(),
        )
    }

    fn default_asset(&self) -> AssetId {
        self.asset_id(&SubAssetId::zeroed())
    }
}

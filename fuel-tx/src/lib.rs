#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::try_err)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![allow(clippy::wrong_self_convention)]
#![deny(clippy::string_slice)]
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
    Bytes32,
    Bytes4,
    Bytes64,
    Bytes8,
    ContractId,
    MessageId,
    Salt,
    Word,
};
pub use tx_pointer::TxPointer;

#[cfg(feature = "builder")]
mod builder;

#[cfg(feature = "alloc")]
mod contract;

#[cfg(feature = "alloc")]
mod receipt;

#[cfg(feature = "alloc")]
mod transaction;

#[cfg(test)]
mod tests;

#[cfg(feature = "builder")]
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
    field,
    input,
    input::Input,
    input::InputRepr,
    Cacheable,
    Chargeable,
    CheckError,
    ConsensusParameters,
    ContractParameters,
    Create,
    DependentCost,
    Executable,
    FeeParameters,
    FormatValidityChecks,
    GasCosts,
    GasCostsValues,
    GasUnit,
    Mint,
    Output,
    OutputRepr,
    PredicateParameters,
    Script,
    ScriptParameters,
    StorageSlot,
    Transaction,
    TransactionFee,
    TransactionRepr,
    TxId,
    TxParameters,
    UtxoId,
    Witness,
};

#[cfg(feature = "std")]
pub use transaction::Signable;
#[cfg(feature = "alloc")]
pub use transaction::UniqueIdentifier;

#[cfg(feature = "alloc")]
#[allow(deprecated)]
pub use transaction::consensus_parameters::default_parameters;

#[cfg(feature = "alloc")]
pub use contract::Contract;

/// Trait extends the functionality of the `ContractId` type.
pub trait ContractIdExt {
    /// Creates an `AssetId` from the `ContractId` and `sub_id`.
    fn asset_id(&self, sub_id: &Bytes32) -> AssetId;
}

impl ContractIdExt for ContractId {
    fn asset_id(&self, sub_id: &Bytes32) -> AssetId {
        let hasher = fuel_crypto::Hasher::default();
        AssetId::new(
            *hasher
                .chain(self.as_slice())
                .chain(sub_id.as_slice())
                .finalize(),
        )
    }
}

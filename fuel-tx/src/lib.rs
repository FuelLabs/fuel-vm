#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::try_err)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![allow(clippy::wrong_self_convention)]

// TODO Add docs

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod consts;

pub use fuel_asm::{InstructionResult, PanicReason};
pub use fuel_types::{
    Address, AssetId, Bytes32, Bytes4, Bytes64, Bytes8, ContractId, MessageId, Salt, Word,
};

#[cfg(feature = "builder")]
mod builder;

#[cfg(feature = "alloc")]
mod contract;

#[cfg(feature = "alloc")]
mod receipt;

#[cfg(feature = "alloc")]
mod transaction;

#[cfg(feature = "std")]
mod checked_transaction;

#[cfg(feature = "builder")]
pub use builder::{Buildable, TransactionBuilder};

#[cfg(feature = "alloc")]
pub use receipt::{Receipt, ScriptExecutionResult};

#[cfg(feature = "alloc")]
pub use transaction::{
    field, Cacheable, Chargeable, CheckError, Checkable, ConsensusParameters, Create, Executable,
    Input, InputRepr, Output, OutputRepr, Script, StorageSlot, Transaction, TransactionFee,
    TransactionRepr, TxId, TxPointer, UtxoId, Witness,
};

#[cfg(feature = "std")]
pub use transaction::{CreateCheckedMetadata, ScriptCheckedMetadata, Signable, UniqueIdentifier};

#[cfg(feature = "alloc")]
#[allow(deprecated)]
pub use transaction::consensus_parameters::default_parameters;

#[cfg(feature = "std")]
pub use checked_transaction::{Checked, CheckedMetadata, CheckedTransaction, IntoChecked};

#[cfg(feature = "alloc")]
pub use contract::Contract;

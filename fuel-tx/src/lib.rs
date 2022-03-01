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
pub use fuel_types::{Address, AssetId, Bytes32, Bytes4, Bytes64, Bytes8, ContractId, Salt, Word};

#[cfg(feature = "builder")]
mod builder;

#[cfg(feature = "alloc")]
mod receipt;

#[cfg(feature = "alloc")]
mod transaction;

#[cfg(feature = "builder")]
pub use builder::TransactionBuilder;

#[cfg(feature = "alloc")]
pub use receipt::Receipt;

#[cfg(feature = "alloc")]
pub use transaction::{
    Input, Metadata, Output, StorageSlot, Transaction, TransactionRepr, TxId, UtxoId,
    ValidationError, Witness,
};

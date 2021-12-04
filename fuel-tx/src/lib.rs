#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::try_err)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![allow(clippy::wrong_self_convention)]

// TODO Add docs

pub mod consts;

pub use fuel_asm::{InstructionResult, PanicReason};
pub use fuel_types::{Address, Bytes32, Bytes4, Bytes64, Bytes8, Color, ContractId, Salt};

#[cfg(feature = "std")]
pub mod crypto;

#[cfg(feature = "std")]
mod transaction;

#[cfg(feature = "std")]
mod receipt;

#[cfg(feature = "std")]
pub use transaction::{Input, Metadata, Output, Transaction, ValidationError, Witness};

#[cfg(feature = "std")]
pub use receipt::Receipt;

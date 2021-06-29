#![allow(clippy::too_many_arguments)]
#![allow(clippy::try_err)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![allow(clippy::wrong_self_convention)]

// TODO Add docs

mod transaction;

pub mod bytes;
pub mod consts;
pub mod crypto;

pub use transaction::{
    Address, Color, ContractId, Hash, Input, Output, Salt, Transaction, ValidationError, Witness,
};

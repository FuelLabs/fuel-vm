// TODO Add docs

mod transaction;

pub mod bytes;
pub mod consts;

pub use transaction::{
    Address, Color, ContractAddress, Hash, Input, Output, Salt, Transaction, ValidationError, Witness,
};

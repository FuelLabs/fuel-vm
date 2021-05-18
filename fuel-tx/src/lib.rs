#![feature(arbitrary_enum_discriminant)]
#![feature(is_sorted)]

// TODO Add docs

mod transaction;

pub mod bytes;
pub mod consts;

pub use transaction::{Color, Id, Input, Output, Root, Transaction, ValidationError, Witness};

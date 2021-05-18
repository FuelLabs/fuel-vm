#![feature(arbitrary_enum_discriminant)]

// TODO Add docs

mod transaction;

pub mod bytes;

pub use transaction::{Color, Id, Input, Output, Root, Transaction, Witness};

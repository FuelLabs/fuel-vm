#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::bool_assert_comparison, clippy::identity_op)]
#![deny(unused_crate_dependencies)]
#![deny(clippy::cast_possible_truncation)]

#[cfg_attr(test, macro_use)]
extern crate alloc;

pub mod binary;
pub mod common;
pub mod sparse;
pub mod storage;

#[cfg(test)]
mod tests;

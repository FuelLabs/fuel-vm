#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::bool_assert_comparison, clippy::identity_op)]
#![deny(unused_crate_dependencies)]
#![deny(
    clippy::arithmetic_side_effects,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::string_slice
)]

#[cfg_attr(test, macro_use)]
extern crate alloc;

pub mod avl;
pub mod binary;
pub mod btree;
pub mod common;
pub mod jellyfish;
pub mod sparse;
pub mod storage;

#[cfg(test)]
mod tests;

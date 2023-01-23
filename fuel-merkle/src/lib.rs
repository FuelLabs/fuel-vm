#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::bool_assert_comparison, clippy::identity_op)]

extern crate alloc;

pub mod binary;
pub mod common;
pub mod sparse;
pub mod storage;
pub mod sum;

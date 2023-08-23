#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::bool_assert_comparison, clippy::identity_op)]
#![deny(unused_crate_dependencies)]

#[cfg_attr(test, macro_use)]
extern crate alloc;

pub mod binary;
pub mod common;
pub mod sparse;
pub mod storage;
pub mod sum;

pub mod error {
    use crate::{
        common::{
            error::DeserializeError,
            node::ChildError,
            Bytes32,
        },
        sparse::StorageNodeError,
    };

    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "std", derive(thiserror::Error))]
    pub enum MerkleTreeError<StorageError> {
        #[cfg_attr(feature = "std", error("proof index {0} is not valid"))]
        InvalidProofIndex(u64),

        #[cfg_attr(
            feature = "std",
            error("cannot load node with key {0}; the key is not found in storage")
        )]
        LoadError64(u64),

        #[cfg_attr(
        feature = "std",
        error("cannot load node with key {}; the key is not found in storage", hex::encode(.0))
        )]
        LoadError32(Bytes32),

        #[cfg_attr(feature = "std", error(transparent))]
        StorageError(StorageError),

        #[cfg_attr(feature = "std", error(transparent))]
        DeserializeError(DeserializeError),

        #[cfg_attr(feature = "std", error(transparent))]
        ChildError(ChildError<Bytes32, StorageNodeError<StorageError>>),

        #[cfg_attr(feature = "std", error("Overflow: {0}"))]
        OverFlow(String),
    }

    impl<StorageError> From<StorageError> for MerkleTreeError<StorageError> {
        fn from(err: StorageError) -> MerkleTreeError<StorageError> {
            MerkleTreeError::StorageError(err)
        }
    }
}

pub use error::MerkleTreeError;

#[cfg(test)]
mod tests;

mod hash;
mod merkle_tree;
mod primitive;

pub(crate) use hash::zero_sum;

pub mod in_memory;
pub mod proof;

use crate::common::Bytes32;

// Define default Merkle Tree structures as concrete implementations of generic
// types, using 32 byte key sizes
pub type MerkleTree<TableType, StorageType> =
    merkle_tree::MerkleTree<32, TableType, StorageType>;
pub type MerkleTreeError<StorageError> = merkle_tree::MerkleTreeError<32, StorageError>;
pub type MerkleTreeKey = merkle_tree::MerkleTreeKey<32>;
pub type Primitive = primitive::Primitive<32>;

pub fn empty_sum() -> &'static Bytes32 {
    zero_sum()
}

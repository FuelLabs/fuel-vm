mod hash;
mod merkle_tree;
mod node;
mod primitive;

pub(crate) use hash::zero_sum;
pub(crate) use node::{StorageNode, StorageNodeError};

pub use merkle_tree::{MerkleTree, MerkleTreeError};
pub use node::Node;
pub use primitive::Primitive;
pub mod branch;
pub mod in_memory;

use crate::common::Bytes32;

pub const fn empty_sum() -> &'static Bytes32 {
    zero_sum()
}

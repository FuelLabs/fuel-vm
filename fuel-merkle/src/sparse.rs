mod hash;
mod merkle_tree;
mod primitive;

pub(crate) use hash::zero_sum;

pub use merkle_tree::{
    MerkleTree,
    MerkleTreeError,
    MerkleTreeKey,
};
pub use primitive::Primitive;
pub mod in_memory;
pub mod proof;

use crate::common::Bytes32;

pub const fn empty_sum() -> &'static Bytes32 {
    zero_sum()
}

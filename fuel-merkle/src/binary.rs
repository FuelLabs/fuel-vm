mod hash;
mod merkle_tree;
mod node;
mod primitive;

pub(crate) use hash::empty_sum;
pub(crate) use hash::{leaf_sum, node_sum};
pub(crate) use node::Node;

pub use merkle_tree::{MerkleTree, MerkleTreeError};
pub use primitive::Primitive;
pub mod in_memory;

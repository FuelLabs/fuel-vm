mod hash;
mod merkle_tree;
mod node;
mod primitive;
mod verify;

pub(crate) use hash::{
    empty_sum,
    node_sum,
};
pub(crate) use node::Node;

pub use hash::leaf_sum;
pub use merkle_tree::{
    MerkleTree,
    MerkleTreeError,
};
pub use primitive::Primitive;
pub mod in_memory;
pub mod root_calculator;

pub use verify::verify;

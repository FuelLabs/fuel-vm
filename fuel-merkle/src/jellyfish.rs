mod merkle_tree;
mod primitive;
mod verify;

pub use merkle_tree::MerkleTreeError;
pub use primitive::Primitive;
pub mod in_memory;

pub use verify::verify;

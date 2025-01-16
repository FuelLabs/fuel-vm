pub mod merkle_tree;

pub use merkle_tree::MerkleTreeError;
pub mod in_memory;

// Re-export dependencies from the jmt crate
// necessary for defining Mappable implementations
// in implementations of the MerkleTree trait.
pub use jmt;

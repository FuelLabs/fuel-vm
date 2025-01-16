/// In Memory implementation of a Jellyfish Merkle Tree.
pub mod in_memory;

/// Integration of jmt::JellyfishMerkleTree with the Storage traits.
pub mod jmt_integration;

/// StorageTrait backed Jellyfish Merkle Tree implementation
pub mod merkle_tree;

pub use merkle_tree::MerkleTreeError;

/// Inclusion and exclusion proofs for the Jellyfish Merkle Tree.
pub mod proof;

// Re-export dependencies from the jmt crate necessary for defining implementations
// of the Mappable trait required by the JellyfishMerkleTree integration.
pub use jmt;

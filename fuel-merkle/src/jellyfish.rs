/// Integration of jmt::JellyfishMerkleTree with the Storage traits.
pub mod jmt_integration;

// Re-export dependencies from the jmt crate necessary for defining implementations
// of the Mappable trait required by the JellyfishMerkleTree integration.
pub use jmt;

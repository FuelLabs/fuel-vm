pub mod generic;
pub mod in_memory;

// Define default Merkle Tree structures as concrete implementations of generic
// types, using 32 byte key sizes
pub type MerkleTree<TableType, StorageType> =
    generic::MerkleTree<32, TableType, StorageType>;
pub type MerkleTreeError<StorageError> = generic::MerkleTreeError<32, StorageError>;
pub type MerkleTreeKey = generic::MerkleTreeKey<32>;
pub type Primitive = generic::Primitive<32>;

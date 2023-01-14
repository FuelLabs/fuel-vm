use crate::{
    common::{Bytes32, StorageMap},
    sparse::{self, Primitive},
};
use fuel_storage::Mappable;

/// The table of the Sparse Merkle tree's nodes. [`MerkleTree`] works with it as a sparse merkle
/// tree, where the storage key is `Bytes32` and the value is the [`Buffer`](crate::sparse::Buffer)
/// (raw presentation of the [`Node`](crate::sparse::Node)).
pub struct NodesTable;

impl Mappable for NodesTable {
    type Key = Bytes32;
    type SetValue = Primitive;
    type GetValue = Self::SetValue;
}

type Storage = StorageMap<NodesTable>;
type SparseMerkleTree = sparse::MerkleTree<NodesTable, Storage>;

pub struct MerkleTree {
    tree: SparseMerkleTree,
}

impl MerkleTree {
    pub fn new() -> Self {
        Self {
            tree: SparseMerkleTree::new(Storage::new()),
        }
    }

    pub fn update(&mut self, key: &Bytes32, data: &[u8]) {
        let _ = self.tree.update(key, data);
    }

    pub fn delete(&mut self, key: &Bytes32) {
        let _ = self.tree.delete(key);
    }

    pub fn root(&self) -> Bytes32 {
        self.tree.root()
    }
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use sparse::hash::sum;

    #[test]
    fn test_empty_root() {
        let tree = MerkleTree::new();
        let root = tree.root();
        let expected_root = "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_1() {
        let mut tree = MerkleTree::new();

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA");

        let root = tree.root();
        let expected_root = "39f36a7cb4dfb1b46f03d044265df6a491dffc1034121bc1071a34ddce9bb14b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_2() {
        let mut tree = MerkleTree::new();

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA");
        tree.update(&sum(b"\x00\x00\x00\x01"), b"DATA");

        let root = tree.root();
        let expected_root = "8d0ae412ca9ca0afcb3217af8bcd5a673e798bd6fd1dfacad17711e883f494cb";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_3() {
        let mut tree = MerkleTree::new();

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA");
        tree.update(&sum(b"\x00\x00\x00\x01"), b"DATA");
        tree.update(&sum(b"\x00\x00\x00\x02"), b"DATA");

        let root = tree.root();
        let expected_root = "52295e42d8de2505fdc0cc825ff9fead419cbcf540d8b30c7c4b9c9b94c268b7";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_1_delete_1() {
        let mut tree = MerkleTree::new();

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA");
        tree.delete(&sum(b"\x00\x00\x00\x00"));

        let root = tree.root();
        let expected_root = "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_2_delete_1() {
        let mut tree = MerkleTree::new();

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA");
        tree.update(&sum(b"\x00\x00\x00\x01"), b"DATA");
        tree.delete(&sum(b"\x00\x00\x00\x01"));

        let root = tree.root();
        let expected_root = "39f36a7cb4dfb1b46f03d044265df6a491dffc1034121bc1071a34ddce9bb14b";
        assert_eq!(hex::encode(root), expected_root);
    }
}

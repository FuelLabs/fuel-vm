use crate::{
    common::{Bytes32, StorageMap},
    sparse::{self, Primitive},
    storage::{Mappable, StorageInspect, StorageMutate},
};
use alloc::borrow::Cow;
use alloc::vec::Vec;

/// The table of the Sparse Merkle tree's nodes. [`MerkleTree`] works with it as a sparse merkle
/// tree, where the storage key is `Bytes32` and the value is the [`Buffer`](crate::sparse::Buffer)
/// (raw presentation of the [`Node`](crate::sparse::Node)).
#[derive(Debug)]
pub struct NodesTable;

impl Mappable for NodesTable {
    type Key = Self::OwnedKey;
    type OwnedKey = Bytes32;
    type Value = Self::OwnedValue;
    type OwnedValue = Primitive;
}

type Storage = StorageMap<NodesTable>;
type SparseMerkleTree = sparse::MerkleTree<NodesTable, Storage>;

/// The safe Merkle tree storage key prevents Merkle tree structure manipulations.
/// The type contains only one constructor that hashes the storage key.
#[derive(Debug, Clone, Copy)]
pub struct MerkleTreeKey(Bytes32);

impl MerkleTreeKey {
    pub fn new<B>(storage_key: B) -> Self
    where
        B: AsRef<[u8]>,
    {
        use digest::Digest;
        let mut hash = sha2::Sha256::new();
        hash.update(storage_key.as_ref());
        let hash = hash
            .finalize()
            .try_into()
            .expect("`sha2::Sha256` can't fail during hashing");

        Self(hash)
    }

    #[cfg(any(test, feature = "test-helpers"))]
    pub fn new_without_hash<B>(storage_key: B) -> Self
    where
        B: Into<Bytes32>,
    {
        Self(storage_key.into())
    }
}

impl From<MerkleTreeKey> for Bytes32 {
    fn from(value: MerkleTreeKey) -> Self {
        value.0
    }
}

#[derive(Debug)]
pub struct MerkleTree {
    tree: SparseMerkleTree,
}

impl MerkleTree {
    pub fn new() -> Self {
        Self {
            tree: SparseMerkleTree::new(Storage::new()),
        }
    }

    pub fn from_set<I, D>(set: I) -> Self
    where
        I: Iterator<Item = (MerkleTreeKey, D)>,
        D: AsRef<[u8]>,
    {
        let tree = SparseMerkleTree::from_set(Storage::new(), set).expect("`Storage` can't return error");
        Self { tree }
    }

    pub fn root_from_set<I, D>(set: I) -> Bytes32
    where
        I: Iterator<Item = (MerkleTreeKey, D)>,
        D: AsRef<[u8]>,
    {
        #[derive(Default)]
        struct EmptyStorage;

        impl StorageInspect<NodesTable> for EmptyStorage {
            type Error = core::convert::Infallible;

            fn get(&self, _: &Bytes32) -> Result<Option<Cow<Primitive>>, Self::Error> {
                Ok(None)
            }

            fn contains_key(&self, _: &Bytes32) -> Result<bool, Self::Error> {
                Ok(false)
            }
        }

        impl StorageMutate<NodesTable> for EmptyStorage {
            fn insert(&mut self, _: &Bytes32, _: &Primitive) -> Result<Option<Primitive>, Self::Error> {
                Ok(None)
            }

            fn remove(&mut self, _: &Bytes32) -> Result<Option<Primitive>, Self::Error> {
                Ok(None)
            }
        }

        let tree = sparse::MerkleTree::<NodesTable, _>::from_set(EmptyStorage::default(), set)
            .expect("`Storage` can't return error");
        tree.root()
    }

    pub fn nodes_from_set<I, D>(set: I) -> (Bytes32, Vec<(Bytes32, Primitive)>)
    where
        I: Iterator<Item = (MerkleTreeKey, D)>,
        D: AsRef<[u8]>,
    {
        #[derive(Default)]
        struct VectorStorage {
            storage: Vec<(Bytes32, Primitive)>,
        }

        impl StorageInspect<NodesTable> for VectorStorage {
            type Error = core::convert::Infallible;

            fn get(&self, _: &Bytes32) -> Result<Option<Cow<Primitive>>, Self::Error> {
                unimplemented!("Read operation is not supported")
            }

            fn contains_key(&self, _: &Bytes32) -> Result<bool, Self::Error> {
                unimplemented!("Read operation is not supported")
            }
        }

        impl StorageMutate<NodesTable> for VectorStorage {
            fn insert(&mut self, key: &Bytes32, value: &Primitive) -> Result<Option<Primitive>, Self::Error> {
                self.storage.push((*key, *value));
                Ok(None)
            }

            fn remove(&mut self, _: &Bytes32) -> Result<Option<Primitive>, Self::Error> {
                unimplemented!("Remove operation is not supported")
            }
        }

        let tree = sparse::MerkleTree::<NodesTable, _>::from_set(VectorStorage::default(), set)
            .expect("`Storage` can't return error");
        let root = tree.root();
        let nodes = tree.into_storage().storage;

        (root, nodes)
    }

    pub fn update(&mut self, key: &MerkleTreeKey, data: &[u8]) {
        let _ = self.tree.update(&key.0, data);
    }

    pub fn delete(&mut self, key: &MerkleTreeKey) {
        let _ = self.tree.delete(&key.0);
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

    fn key(data: &[u8]) -> MerkleTreeKey {
        MerkleTreeKey::new_without_hash(sum(data))
    }

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

        tree.update(&key(b"\x00\x00\x00\x00"), b"DATA");

        let root = tree.root();
        let expected_root = "39f36a7cb4dfb1b46f03d044265df6a491dffc1034121bc1071a34ddce9bb14b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_2() {
        let mut tree = MerkleTree::new();

        tree.update(&key(b"\x00\x00\x00\x00"), b"DATA");
        tree.update(&key(b"\x00\x00\x00\x01"), b"DATA");

        let root = tree.root();
        let expected_root = "8d0ae412ca9ca0afcb3217af8bcd5a673e798bd6fd1dfacad17711e883f494cb";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_3() {
        let mut tree = MerkleTree::new();

        tree.update(&key(b"\x00\x00\x00\x00"), b"DATA");
        tree.update(&key(b"\x00\x00\x00\x01"), b"DATA");
        tree.update(&key(b"\x00\x00\x00\x02"), b"DATA");

        let root = tree.root();
        let expected_root = "52295e42d8de2505fdc0cc825ff9fead419cbcf540d8b30c7c4b9c9b94c268b7";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_1_delete_1() {
        let mut tree = MerkleTree::new();

        tree.update(&key(b"\x00\x00\x00\x00"), b"DATA");
        tree.delete(&key(b"\x00\x00\x00\x00"));

        let root = tree.root();
        let expected_root = "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_2_delete_1() {
        let mut tree = MerkleTree::new();

        tree.update(&key(b"\x00\x00\x00\x00"), b"DATA");
        tree.update(&key(b"\x00\x00\x00\x01"), b"DATA");
        tree.delete(&key(b"\x00\x00\x00\x01"));

        let root = tree.root();
        let expected_root = "39f36a7cb4dfb1b46f03d044265df6a491dffc1034121bc1071a34ddce9bb14b";
        assert_eq!(hex::encode(root), expected_root);
    }
}

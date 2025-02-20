use core::borrow::Borrow;

use crate::{
    common::Bytes32,
    merkle_patricia_trie,
    sparse::{
        self,
        proof::Proof,
        MerkleTreeKey,
        Primitive,
    },
    storage::{
        Mappable,
        StorageInspect,
        StorageMutate,
    },
};
use alloc::{
    borrow::Cow,
    vec::Vec,
};
use alloy_trie::nodes::{
    RlpNode,
    TrieNode,
};
use nybbles::Nibbles;

use super::storage_map::StorageMap;

#[derive(Debug, Clone, Eq, PartialEq)]
struct WrappedRlpNode(RlpNode);

impl core::hash::Hash for WrappedRlpNode {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl From<RlpNode> for WrappedRlpNode {
    fn from(node: RlpNode) -> Self {
        Self(node)
    }
}

impl From<WrappedRlpNode> for RlpNode {
    fn from(node: WrappedRlpNode) -> Self {
        node.0
    }
}

impl Borrow<RlpNode> for WrappedRlpNode {
    fn borrow(&self) -> &RlpNode {
        &self.0
    }
}

/// The table of the Sparse Merkle tree's nodes. [`MerkleTree`] works with it as a sparse
/// merkle tree, where the storage key is `Bytes32` and the value is the
/// [`Buffer`](crate::sparse::Buffer) (raw presentation of the
/// [`Node`](crate::sparse::Node)).
#[derive(Debug)]
pub struct NodesTable;

//     NodesTableType: Mappable<Key = RlpNode, Value = TrieNode, OwnedValue = TrieNode>,

impl Mappable for NodesTable {
    type Key = Self::OwnedKey;
    type OwnedKey = RlpNode;
    type OwnedValue = TrieNode;
    type Value = Self::OwnedValue;
}

type Storage = StorageMap<NodesTable, WrappedRlpNode>;
type Trie = merkle_patricia_trie::trie::Trie<Storage, NodesTable>;

#[derive(Debug)]
pub struct MerklePatriciaTrie {
    trie: Trie,
}

impl MerklePatriciaTrie {
    pub fn new() -> Self {
        Self {
            trie: Trie::new(Storage::new()),
        }
    }

    /// Build a sparse Merkle tree from a set of key-value pairs. This is
    /// equivalent to creating an empty sparse Merkle tree and sequentially
    /// calling [update](Self::update) for each key-value pair. This constructor
    /// is more performant than calling individual sequential updates and is the
    /// preferred approach when the key-values are known upfront. Leaves can be
    /// appended to the returned tree using `update` to further accumulate leaf
    /// data.
    pub fn from_set<I, D>(set: I) -> anyhow::Result<Self>
    where
        I: Iterator<Item = (MerkleTreeKey, D)>,
        D: AsRef<[u8]>,
    {
        // TODO: Optimize this using the alloy primitives to construct a tree from a set
        // ok key value pairs. This will also be needed to validate whether our
        // implementation is consistent with the alloy implementation.
        let mut trie = MerklePatriciaTrie::new();
        for (key, value) in set {
            let value: &[u8] = value.as_ref();
            trie.trie.add_leaf(*key, value)?;
        }

        Ok(trie)
    }

    /// Calculate the sparse Merkle root from a set of key-value pairs. This is
    /// similar to constructing a new tree from a set of key-value pairs using
    /// [from_set](Self::from_set), except this method returns only the root; it
    /// does not write to storage nor return a sparse Merkle tree instance. It
    /// is equivalent to calling `from_set(..)`, followed by `root()`, but does
    /// not incur the overhead of storage writes. This can be helpful when we
    /// know all the key-values in the set upfront and we will not need to
    /// update the set in the future.
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
            fn insert(&mut self, _: &Bytes32, _: &Primitive) -> Result<(), Self::Error> {
                Ok(())
            }

            fn replace(
                &mut self,
                _: &Bytes32,
                _: &Primitive,
            ) -> Result<Option<Primitive>, Self::Error> {
                Ok(None)
            }

            fn remove(&mut self, _: &Bytes32) -> Result<(), Self::Error> {
                Ok(())
            }

            fn take(&mut self, _: &Bytes32) -> Result<Option<Primitive>, Self::Error> {
                Ok(None)
            }
        }

        let tree = sparse::MerkleTree::<NodesTable, _>::from_set(EmptyStorage, set)
            .expect("`Storage` can't return error");
        tree.root()
    }

    /// Calculate the sparse Merkle root as well as all nodes in the Merkle tree
    /// from a set of key-value pairs. This is similar to constructing a new
    /// tree from a set of key-value pairs using [from_set](Self::from_set),
    /// except this method returns only the root and the list of leaves and
    /// nodes in the tree; it does not return a sparse Merkle tree instance.
    /// This can be helpful when we know all the key-values in the set upfront
    /// and we need to defer storage writes, such as expensive database inserts,
    /// for batch operations later in the process.
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
            fn insert(
                &mut self,
                key: &Bytes32,
                value: &Primitive,
            ) -> Result<(), Self::Error> {
                self.storage.push((*key, *value));
                Ok(())
            }

            fn replace(
                &mut self,
                key: &Bytes32,
                value: &Primitive,
            ) -> Result<Option<Primitive>, Self::Error> {
                self.storage.push((*key, *value));
                Ok(None)
            }

            fn remove(&mut self, _: &Bytes32) -> Result<(), Self::Error> {
                unimplemented!("Remove operation is not supported")
            }

            fn take(&mut self, _: &Bytes32) -> Result<Option<Primitive>, Self::Error> {
                unimplemented!("Take operation is not supported")
            }
        }

        let tree =
            sparse::MerkleTree::<NodesTable, _>::from_set(VectorStorage::default(), set)
                .expect("`Storage` can't return error");
        let root = tree.root();
        let nodes = tree.into_storage().storage;

        (root, nodes)
    }

    pub fn update(&mut self, key: MerkleTreeKey, data: &[u8]) {
        let _ = self.tree.update(key, data);
    }

    pub fn delete(&mut self, key: MerkleTreeKey) {
        let _ = self.tree.delete(key);
    }

    pub fn root(&self) -> Bytes32 {
        self.tree.root()
    }

    pub fn generate_proof(&self, key: &MerkleTreeKey) -> Option<Proof> {
        self.tree.generate_proof(key).ok()
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
    use crate::common::sum;

    fn key(data: &[u8]) -> MerkleTreeKey {
        MerkleTreeKey::new_without_hash(sum(data))
    }

    #[test]
    fn test_empty_root() {
        let tree = MerkleTree::new();
        let root = tree.root();
        let expected_root =
            "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_1() {
        let mut tree = MerkleTree::new();

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA");

        let root = tree.root();
        let expected_root =
            "39f36a7cb4dfb1b46f03d044265df6a491dffc1034121bc1071a34ddce9bb14b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_2() {
        let mut tree = MerkleTree::new();

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA");
        tree.update(key(b"\x00\x00\x00\x01"), b"DATA");

        let root = tree.root();
        let expected_root =
            "8d0ae412ca9ca0afcb3217af8bcd5a673e798bd6fd1dfacad17711e883f494cb";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_3() {
        let mut tree = MerkleTree::new();

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA");
        tree.update(key(b"\x00\x00\x00\x01"), b"DATA");
        tree.update(key(b"\x00\x00\x00\x02"), b"DATA");

        let root = tree.root();
        let expected_root =
            "52295e42d8de2505fdc0cc825ff9fead419cbcf540d8b30c7c4b9c9b94c268b7";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_1_delete_1() {
        let mut tree = MerkleTree::new();

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA");
        tree.delete(key(b"\x00\x00\x00\x00"));

        let root = tree.root();
        let expected_root =
            "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_2_delete_1() {
        let mut tree = MerkleTree::new();

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA");
        tree.update(key(b"\x00\x00\x00\x01"), b"DATA");
        tree.delete(key(b"\x00\x00\x00\x01"));

        let root = tree.root();
        let expected_root =
            "39f36a7cb4dfb1b46f03d044265df6a491dffc1034121bc1071a34ddce9bb14b";
        assert_eq!(hex::encode(root), expected_root);
    }
}

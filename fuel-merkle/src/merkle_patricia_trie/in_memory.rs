use core::borrow::Borrow;

use crate::{
    merkle_patricia_trie,
    sparse::{
        proof::Proof,
        MerkleTreeKey,
    },
    storage::Mappable,
};
use alloc::vec::Vec;
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

    pub fn root_from_set<I, D>(set: I) -> RlpNode
    where
        I: Iterator<Item = (MerkleTreeKey, D)>,
        D: AsRef<[u8]>,
    {
        // TODO: Optimize this
        let trie =
            MerklePatriciaTrie::from_set(set).expect("`Storage` can't return error");
        trie.trie.root()
    }

    /// Calculate the sparse Merkle root as well as all nodes in the Merkle tree
    /// from a set of key-value pairs. This is similar to constructing a new
    /// tree from a set of key-value pairs using [from_set](Self::from_set),
    /// except this method returns only the root and the list of leaves and
    /// nodes in the tree; it does not return a sparse Merkle tree instance.
    /// This can be helpful when we know all the key-values in the set upfront
    /// and we need to defer storage writes, such as expensive database inserts,
    /// for batch operations later in the process.
    pub fn nodes_from_set<I, D>(set: I) -> (RlpNode, Vec<(RlpNode, TrieNode)>)
    where
        I: Iterator<Item = (MerkleTreeKey, D)>,
        D: AsRef<[u8]>,
    {
        let trie = Self::from_set(set).expect("Storage can't return error");

        let root = trie.trie.root();
        let nodes = trie.trie.storage.nodes();

        (root, nodes)
    }

    pub fn update(&mut self, key: MerkleTreeKey, data: &[u8]) {
        let _ = self.trie.delete_leaf(&Nibbles::from_nibbles(key.as_ref()));
        let _ = self.trie.add_leaf(*key, data);
    }

    pub fn delete(&mut self, key: MerkleTreeKey) {
        let _ = self.trie.delete_leaf(&Nibbles::from_nibbles(key.as_ref()));
    }

    pub fn root(&self) -> RlpNode {
        self.trie.root()
    }

    pub fn generate_proof(&self, _key: &MerkleTreeKey) -> Option<Proof> {
        todo!()
    }
}

impl Default for MerklePatriciaTrie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}

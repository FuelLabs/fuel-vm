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
        let _ = self.trie.delete_leaf(&Nibbles::unpack(key.as_ref()));
        let _ = self.trie.add_leaf(*key, data);
    }

    pub fn delete(&mut self, key: MerkleTreeKey) {
        let _ = self.trie.delete_leaf(&Nibbles::unpack(key.as_ref()));
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
    use alloy_trie::nodes::TrieNode;
    use nybbles::Nibbles;

    use crate::{
        merkle_patricia_trie::in_memory::WrappedRlpNode,
        sparse::MerkleTreeKey,
    };

    #[test]
    fn empty_trie_returns_empty_root() {
        let trie = super::MerklePatriciaTrie::new();
        let root = trie.root();
        let expected = TrieNode::EmptyRoot.rlp(&mut Vec::with_capacity(33));
        assert_eq!(root, expected);
    }

    #[test]
    fn add_leaf_adds_extension_node() {
        let mut trie = super::MerklePatriciaTrie::new();
        let key = MerkleTreeKey::new_without_hash([0; 32]);
        trie.update(key, b"DATA");
        let root_rlp = trie.root();
        let nodes = trie.trie.storage.nodes();
        // One leaf node and one extension node
        assert_eq!(nodes.len(), 2);
        let (extension_node_rlp, extension_node) = nodes
            .iter()
            .find(|(_, node)| matches!(node, TrieNode::Extension(_)))
            .expect("Trie should have an extension node");

        let (leaf_node_rlp, leaf_node) = nodes
            .iter()
            .find(|(_, node)| matches!(node, TrieNode::Leaf(_)))
            .expect("Trie should have a leaf node");

        let expected_extension_node_rlp = extension_node.rlp(&mut Vec::with_capacity(33));
        let expected_leaf_node_rlp = leaf_node.rlp(&mut Vec::with_capacity(33));

        assert_eq!(*extension_node_rlp, expected_extension_node_rlp);
        assert_eq!(*leaf_node_rlp, expected_leaf_node_rlp);

        let (extension_node_key, extension_node_child) = match extension_node {
            TrieNode::Extension(node) => (&*node.key.pack(), &node.child),
            _ => panic!("Not an extension node"),
        };
        assert_eq!(extension_node_key, &[0u8; 32]);
        assert_eq!(extension_node_child, leaf_node_rlp);

        assert_eq!(&root_rlp, extension_node_rlp);
    }

    #[test]
    fn update_and_delete() {
        let mut trie = super::MerklePatriciaTrie::new();
        let key = MerkleTreeKey::new_without_hash([0; 32]);
        trie.update(key, b"DATA");
        trie.delete(key);
        let root_rlp = trie.root();
        let expected_root = super::TrieNode::EmptyRoot.rlp(&mut Vec::with_capacity(33));
        let nodes = trie.trie.storage.nodes();
        assert_eq!(nodes.len(), 1);
        let (storage_root_rlp, storage_root) = nodes.get(0).unwrap();

        assert_eq!(storage_root_rlp, &root_rlp);
        assert_eq!(storage_root, &TrieNode::EmptyRoot);
        assert_eq!(root_rlp, expected_root);
    }

    #[test]
    fn add_two_nodes_with_branch_node_at_root() {
        let mut trie = super::MerklePatriciaTrie::new();
        let key1 = MerkleTreeKey::new_without_hash([0; 32]);
        let key2 = MerkleTreeKey::new_without_hash([17; 32]);
        trie.update(key1, b"DATA1");
        trie.update(key2, b"DATA2");
        let root_rlp = trie.root();
        let storage = trie.trie.storage;
        let nodes = storage.nodes();
        // One branch node,  two extension nodes, and two leaf nodes
        assert_eq!(nodes.len(), 5);

        let num_branch_nodes = nodes
            .iter()
            .filter(|(_, node)| matches!(node, TrieNode::Branch(_)))
            .count();

        assert_eq!(num_branch_nodes, 1);

        let (branch_node_rlp, branch_node) = nodes
            .iter()
            .find(|(_, node)| matches!(node, TrieNode::Branch(_)))
            .expect("Trie should have a branch node");

        assert_eq!(branch_node_rlp, &root_rlp);
        let TrieNode::Branch(branch_node) = branch_node else {
            unreachable!()
        };
        let branch_node_ref = branch_node.as_ref();
        let mut children = branch_node_ref
            .children()
            .filter_map(|(nibble, node)| node.map(|node| (nibble, node)));
        let Some((nibble_0, extension_node_0_rlp)) = children.next() else {
            panic!("No child node")
        };
        let Some(TrieNode::Extension(extension_node_0)) = storage
            .map
            .get(&WrappedRlpNode(extension_node_0_rlp.clone()))
        else {
            panic!("Extension node not in storage")
        };
        let leaf_0_rlp = &extension_node_0.child;
        let Some(TrieNode::Leaf(leaf_0)) =
            storage.map.get(&WrappedRlpNode(leaf_0_rlp.clone()))
        else {
            panic!("Leaf node not in storage")
        };

        // Extension node 0 should have only 63 bits set
        assert_eq!(nibble_0, 0);
        assert_eq!(extension_node_0.key.as_ref(), &[0u8; 63]);
        assert_eq!(leaf_0.value, b"DATA1");

        let Some((nibble_1, extension_node_1_rlp)) = children.next() else {
            panic!("No child node")
        };
        assert_eq!(nibble_1, 1);
        let Some(TrieNode::Extension(extension_node_1)) = storage
            .map
            .get(&WrappedRlpNode(extension_node_1_rlp.clone()))
        else {
            panic!("Extension node not in storage")
        };
        let leaf_1_rlp = &extension_node_1.child;
        let Some(TrieNode::Leaf(leaf_1)) =
            storage.map.get(&WrappedRlpNode(leaf_1_rlp.clone()))
        else {
            panic!("Leaf node not in storage")
        };

        // Extension node 0 should have only 63 bits set
        assert_eq!(nibble_1, 1);
        assert_eq!(extension_node_1.key.as_ref(), &[1u8; 63]);
        assert_eq!(leaf_1.value, b"DATA2");
    }

    #[test]
    fn add_two_nodes_with_branch_node_before_leaf() {
        let mut trie = super::MerklePatriciaTrie::new();
        let key1 = MerkleTreeKey::new_without_hash([0; 32]);
        let mut raw_key2 = [0; 32];
        raw_key2[31] = 1; // two nibbles: 0x0 and 0x1.
        let key2 = MerkleTreeKey::new_without_hash(raw_key2.clone());
        trie.update(key1, b"DATA1");
        trie.update(key2, b"DATA2");
        let root_rlp = trie.root();
        let storage = trie.trie.storage.map;

        // One extension node, One branch node,  two leaf nodes
        assert_eq!(storage.len(), 4);

        let Some(TrieNode::Extension(root_node)) = storage.get(&WrappedRlpNode(root_rlp))
        else {
            panic!("Root node not in storage");
        };
        assert_eq!(root_node.key.as_ref(), &[0u8; 63]);
        let branch_node_rlp = &root_node.child;
        let Some(TrieNode::Branch(branch_node)) =
            storage.get(&WrappedRlpNode(branch_node_rlp.clone()))
        else {
            panic!("Branch node not in storage");
        };
        let branch_node_ref = branch_node.as_ref();
        let mut children = branch_node_ref
            .children()
            .filter_map(|(nibble, node)| node.map(|node| (nibble, node)));
        let Some((nibble_0, leaf_0_rlp)) = children.next() else {
            panic!("No child node")
        };
        let Some(TrieNode::Leaf(leaf_0)) =
            storage.get(&WrappedRlpNode(leaf_0_rlp.clone()))
        else {
            panic!("Leaf node not in storage")
        };
        assert_eq!(nibble_0, 0);
        assert_eq!(leaf_0.key.as_ref(), &[0u8; 64]);
        assert_eq!(leaf_0.value, b"DATA1");

        let Some((nibble_1, leaf_1_rlp)) = children.next() else {
            panic!("No child node")
        };
        let Some(TrieNode::Leaf(leaf_1)) =
            storage.get(&WrappedRlpNode(leaf_1_rlp.clone()))
        else {
            panic!("Leaf node not in storage")
        };
        assert_eq!(nibble_1, 1);
        assert_eq!(leaf_1.key, Nibbles::unpack(&raw_key2));
        assert_eq!(leaf_1.value, b"DATA2");

        assert_eq!(children.next(), None);
    }

    #[test]
    fn add_two_nodes_with_branch_node_in_middle() {
        let mut trie = super::MerklePatriciaTrie::new();
        let key1 = MerkleTreeKey::new_without_hash([0; 32]);
        let mut raw_key2 = [0; 32];
        raw_key2[0] = 1; // two nibbles: 0x0 and 0x1.
        let key2 = MerkleTreeKey::new_without_hash(raw_key2.clone());
        trie.update(key1, b"DATA1");
        trie.update(key2, b"DATA2");
        let root_rlp = trie.root();
        let storage = trie.trie.storage.map;

        // One extension node, One branch node,  two extension nodes, two leaf nodes
        assert_eq!(storage.len(), 6);

        let Some(TrieNode::Extension(root_node)) = storage.get(&WrappedRlpNode(root_rlp))
        else {
            panic!("Root node not in storage");
        };
        assert_eq!(root_node.key.as_ref(), &[0u8; 1]);
        let branch_node_rlp = &root_node.child;
        let Some(TrieNode::Branch(branch_node)) =
            storage.get(&WrappedRlpNode(branch_node_rlp.clone()))
        else {
            panic!("Branch node not in storage");
        };
        let branch_node_ref = branch_node.as_ref();
        let mut children = branch_node_ref
            .children()
            .filter_map(|(nibble, node)| node.map(|node| (nibble, node)));
        let Some((nibble_0, extension_node_0_rlp)) = children.next() else {
            panic!("No child node")
        };
        let Some(TrieNode::Extension(extension_node_0)) =
            storage.get(&WrappedRlpNode(extension_node_0_rlp.clone()))
        else {
            panic!("Extension node not in storage")
        };
        let leaf_0_rlp = &extension_node_0.child;
        let Some(TrieNode::Leaf(leaf_0)) =
            storage.get(&WrappedRlpNode(leaf_0_rlp.clone()))
        else {
            panic!("Leaf node not in storage")
        };

        assert_eq!(nibble_0, 0);
        assert_eq!(extension_node_0.key.as_ref(), &[0u8; 62]);
        assert_eq!(leaf_0.key.as_ref(), &[0u8; 64]);
        assert_eq!(leaf_0.value, b"DATA1");

        let Some((nibble_1, extension_node_1_rlp)) = children.next() else {
            panic!("No child node")
        };

        let Some(TrieNode::Extension(extension_node_1)) =
            storage.get(&WrappedRlpNode(extension_node_1_rlp.clone()))
        else {
            panic!("Extension node not in storage")
        };
        let leaf_1_rlp = &extension_node_1.child;
        let Some(TrieNode::Leaf(leaf_1)) =
            storage.get(&WrappedRlpNode(leaf_1_rlp.clone()))
        else {
            panic!("Leaf node not in storage")
        };

        assert_eq!(nibble_1, 1);
        assert_eq!(extension_node_1.key.as_ref(), &[0u8; 62]);
        assert_eq!(leaf_1.key, Nibbles::unpack(&raw_key2));
        assert_eq!(leaf_1.value, b"DATA2");

        assert_eq!(children.next(), None);
    }

    #[test]
    fn branch_three_nodes() {
        let mut trie = super::MerklePatriciaTrie::new();
        let key1 = MerkleTreeKey::new_without_hash([0x00; 32]);
        let key2 = MerkleTreeKey::new_without_hash([0x11; 32]);
        let key3 = MerkleTreeKey::new_without_hash([0x88; 32]);
        trie.update(key1, b"DATA1");
        trie.update(key2, b"DATA2");
        trie.update(key3, b"DATA3");
        let root_rlp = trie.root();
        let storage = trie.trie.storage.map;

        // One branch node,  three extension nodes, three leaf nodes
        assert_eq!(storage.len(), 7);

        let Some(TrieNode::Branch(root_node)) = storage.get(&WrappedRlpNode(root_rlp))
        else {
            panic!("Root node not in storage");
        };

        let branch_node_ref = root_node.as_ref();
        let mut children = branch_node_ref
            .children()
            .filter_map(|(nibble, node)| node.map(|node| (nibble, node)));

        let (nibble_0, extension_node_0_rlp) = children.next().expect("No child node");
        let (nibble_1, extension_node_1_rlp) = children.next().expect("No child node");
        let (nibble_8, extension_node_8_rlp) = children.next().expect("No child node");

        let Some(TrieNode::Extension(extension_node_0)) =
            storage.get(&WrappedRlpNode(extension_node_0_rlp.clone()))
        else {
            panic!("Extension node not in storage")
        };
        let leaf_0_rlp = &extension_node_0.child;
        let Some(TrieNode::Leaf(leaf_0)) =
            storage.get(&WrappedRlpNode(leaf_0_rlp.clone()))
        else {
            panic!("Leaf node not in storage")
        };

        let Some(TrieNode::Extension(extension_node_1)) =
            storage.get(&WrappedRlpNode(extension_node_1_rlp.clone()))
        else {
            panic!("Extension node not in storage")
        };
        let leaf_1_rlp = &extension_node_1.child;
        let Some(TrieNode::Leaf(leaf_1)) =
            storage.get(&WrappedRlpNode(leaf_1_rlp.clone()))
        else {
            panic!("Leaf node not in storage")
        };

        let Some(TrieNode::Extension(extension_node_8)) =
            storage.get(&WrappedRlpNode(extension_node_8_rlp.clone()))
        else {
            panic!("Extension node not in storage")
        };
        let leaf_8_rlp = &extension_node_8.child;
        let Some(TrieNode::Leaf(leaf_8)) =
            storage.get(&WrappedRlpNode(leaf_8_rlp.clone()))
        else {
            panic!("Leaf node not in storage")
        };

        println!("");
        println!("{:?}", storage);

        assert_eq!(nibble_0, 0);
        assert_eq!(extension_node_0.key.as_ref(), &[0x0; 63]);
        assert_eq!(leaf_0.key.as_ref(), &[0x0; 64]);
        assert_eq!(leaf_0.value, b"DATA1");

        assert_eq!(nibble_1, 1);
        assert_eq!(extension_node_1.key.as_ref(), &[0x1; 63]);
        assert_eq!(leaf_1.key.as_ref(), &[0x1; 64]);
        assert_eq!(leaf_1.value, b"DATA2");

        assert_eq!(nibble_8, 8);
        assert_eq!(extension_node_8.key.as_ref(), &[0x8; 63]);
        assert_eq!(leaf_8.key.as_ref(), &[0x8; 64]);
        assert_eq!(leaf_8.value, b"DATA3");

        assert_eq!(children.next(), None);
    }

    #[test]
    fn update_existing_leaf() {
        let mut trie = super::MerklePatriciaTrie::new();
        let key = MerkleTreeKey::new_without_hash([0x00; 32]);
        trie.update(key, b"DATA1");
        trie.update(key, b"DATA2");

        let root_rlp = trie.root();
        let storage = trie.trie.storage.map;

        println!("");
        println!("{:?}", storage);
        // One extension node, One leaf node
        assert_eq!(storage.len(), 2);

        let Some(TrieNode::Extension(root_node)) = storage.get(&WrappedRlpNode(root_rlp))
        else {
            panic!("Root node not in storage");
        };

        let extension_node_key = &root_node.key;
        let leaf_node_rlp = &root_node.child;
        let Some(TrieNode::Leaf(leaf_node)) =
            storage.get(&WrappedRlpNode(leaf_node_rlp.clone()))
        else {
            panic!("Leaf node not in storage")
        };

        assert_eq!(extension_node_key.as_ref(), &[0u8; 64]);
        assert_eq!(leaf_node.key.as_ref(), &[0u8; 64]);
        assert_eq!(leaf_node.value, b"DATA2");
    }
}

use core::marker::PhantomData;

use alloy_trie::nodes::{
    BranchNode,
    RlpNode,
    TrieNode,
};

use alloy_primitives::B256;
use fuel_storage::{
    Mappable,
    StorageInspect,
    StorageMutate,
};
use nybbles::Nibbles;

use crate::common::Bytes32;

pub struct Trie<Storage, NodesTable> {
    storage: Storage,
    root: B256,
    _phantom: PhantomData<NodesTable>,
}

impl<Storage, NodesTableType> Trie<Storage, NodesTableType> {
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
            root: B256::ZERO,
            _phantom: PhantomData,
        }
    }
}

impl<StorageType, NodesTableType> Trie<StorageType, NodesTableType>
where
    StorageType: StorageMutate<NodesTableType, Error = anyhow::Error>,
    NodesTableType: Mappable<Key = RlpNode, Value = TrieNode, OwnedValue = TrieNode>,
{
}

// Iterator for traversing a trie node with respect to a Nibble path
struct NodeIterator<'a, StorageType, NodesTableType> {
    nibbles_left: &'a [u8],
    current_node: Option<RlpNode>,
    storage: StorageType,
    _marker: PhantomData<NodesTableType>,
}

impl<'a, StorageType, NodesTableType> Iterator
    for NodeIterator<'a, StorageType, NodesTableType>
where
    StorageType: StorageMutate<NodesTableType, Error = anyhow::Error>,
    NodesTableType: Mappable<Key = RlpNode, Value = TrieNode, OwnedValue = TrieNode>,
{
    type Item = anyhow::Result<TrieNode>;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(current_rlp_node) = self.current_node.take() else {
            return None
        };
        let node = self.storage.get(&current_rlp_node);
        match node {
            Err(e) => Some(Err(e)),
            Ok(None) => Some(Err(anyhow::anyhow!("Node referenced but not found"))),
            Ok(Some(node)) => {
                let owned_node = node.into_owned();
                match &owned_node {
                    TrieNode::EmptyRoot => {
                        // This can happen if we have the whole tree is empty.
                        // There is no next node in the path
                        self.nibbles_left = &[];
                        self.current_node = None;
                    }
                    TrieNode::Branch(branch_node) => {
                        // Branch node: we can look at the first nibble, and
                        // select the next node based on its value.
                        let Some((next_nibble, nibbles_left)) =
                            self.nibbles_left.split_first()
                        else {
                            self.current_node = None;
                            return Some(Ok(owned_node));
                        };
                        let branch_node_ref = branch_node.as_ref();
                        let next_node = branch_node_ref
                            .children()
                            .filter(|(nibble, _node)| (nibble == next_nibble))
                            .collect::<Vec<_>>()[0]
                            .1;
                        self.nibbles_left = nibbles_left;
                        self.current_node = next_node.cloned();
                    }
                    TrieNode::Extension(extension_node) => {
                        // Check if the nibbles left are a prefix of the extension node
                        // nibbles. If so, remove them from the
                        // nibbles left and load the next node.
                        // Othewise, there is no next node in the traversal
                        let extension_node_ref = extension_node.as_ref();
                        let extension_node_nibbles: &[u8] = extension_node_ref.key;
                        if self.nibbles_left.starts_with(extension_node_nibbles) {
                            self.nibbles_left =
                                &self.nibbles_left[extension_node_nibbles.len()..];
                            self.current_node = Some(extension_node.child.clone());
                        } else {
                            self.nibbles_left = &[];
                            self.current_node = None;
                        }
                    }
                    TrieNode::Leaf(leaf_node) => {
                        self.nibbles_left = &[];
                        self.current_node = None;
                    }
                };
                Some(Ok(owned_node))
            }
        }
    }
}

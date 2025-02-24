use core::marker::PhantomData;

use alloy_trie::nodes::{
    BranchNode,
    ExtensionNode,
    LeafNode,
    RlpNode,
    TrieNode,
};
use fuel_storage::{
    Mappable,
    StorageInspect,
};
use nybbles::Nibbles;

// Iterator for traversing a trie node with respect to a Nibble path
// The nibble path must be obtained from a leaf key, e.g. consist of
// exactly 64 nibbles. This guarantees that we never reach a branch
// node as the last node in the path, and we can return the nibble
// that will be used to select a node together with a branch node.
pub struct NodeIterator<'a, 'b, StorageType, NodesTableType> {
    pub(crate) nibbles_left: &'a [u8],
    pub(crate) current_node: Option<RlpNode>,
    pub(crate) storage: &'b StorageType,
    pub(crate) _marker: PhantomData<NodesTableType>,
}

impl<StorageType, NodesTableType> NodeIterator<'_, '_, StorageType, NodesTableType> {
    pub fn nibbles_left(&self) -> Nibbles {
        Nibbles::from_nibbles(self.nibbles_left)
    }
}

#[derive(Debug)]
pub enum TraversedNode {
    EmptyRoot(RlpNode),
    Leaf(RlpNode, LeafNode),
    // The branch node, the branch node itself, and the nibble that will be used to
    // select the next node.
    Branch(RlpNode, BranchNode, u8),
    Extension(RlpNode, ExtensionNode),
}

impl<StorageType, NodesTableType> Iterator
    for NodeIterator<'_, '_, StorageType, NodesTableType>
where
    StorageType: StorageInspect<NodesTableType>,
    NodesTableType: Mappable<Key = RlpNode, Value = TrieNode, OwnedValue = TrieNode>,
{
    // Return the next node, and the nibble that will be used to select the next node,
    // if any.
    type Item = anyhow::Result<TraversedNode>;

    fn next(&mut self) -> Option<Self::Item> {
        let current_rlp_node = self.current_node.take()?;
        println!("Iterator - current_rlp_node: {:?}", current_rlp_node);
        // Check if we got the empty root node, in which case we can avoid
        // fetching it from the storage.
        if &*current_rlp_node == &[128] {
            println!("Iterator - Empty root node found");
            self.current_node = None;
            return Some(Ok(TraversedNode::EmptyRoot(
                RlpNode::from_raw(&[128]).unwrap(),
            )));
        }
        let node = self.storage.get(&current_rlp_node);
        match node {
            Err(_e) => {
                println!("Error while fetching node from storage");
                Some(Err(anyhow::anyhow!(
                    "Node {:?} could not be loaded",
                    current_rlp_node
                )))
            }
            Ok(None) => {
                println!("Node not found in storage");
                Some(Err(anyhow::anyhow!(
                    "Node {:?} referenced but not present in storage",
                    current_rlp_node
                )))
            }
            Ok(Some(node)) => {
                println!("Node found in strorage: {:?}", node.as_ref());
                match node.as_ref() {
                    TrieNode::EmptyRoot => {
                        // This can happen if we have the whole tree is empty.
                        // There is no next node in the path
                        // We do not update the nibbles left. This is useful
                        // when inserting a new node, as we can use the nibbles left
                        // ti identify the path to the node to be inserted.
                        self.current_node = None;
                        Some(Ok(TraversedNode::EmptyRoot(current_rlp_node)))
                    }
                    TrieNode::Branch(branch_node) => {
                        // Branch node: we can look at the first nibble, and
                        // select the next node based on its value.
                        let Some((next_nibble, nibbles_left)) =
                            self.nibbles_left.split_first()
                        else {
                            // Technically here we encountered a branch node that we never
                            // return in the iterator, but I guess this is okay since this
                            // scenario should not be possible in the current
                            // implementation.
                            self.current_node = None;
                            return Some(Err(anyhow::anyhow!(
                                "Branch node at the end of path, no nibbles left"
                            )));
                        };
                        let branch_node_ref = branch_node.as_ref();
                        let next_node = branch_node_ref
                            .children()
                            .find(|(nibble, _node)| (nibble == next_nibble))
                            // Guaranteed to exist because the nibble
                            .unwrap()
                            .1;
                        self.nibbles_left = nibbles_left;
                        self.current_node = next_node.cloned();
                        Some(Ok(TraversedNode::Branch(
                            current_rlp_node,
                            branch_node.clone(),
                            *next_nibble,
                        )))
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
                            // Do not update the nibbles left, as this information
                            // is needed when inserting a new leaf.
                            self.current_node = None;
                        };
                        // We return the extension node irrespsective of whether it
                        // can be traversed completely or not.
                        Some(Ok(TraversedNode::Extension(
                            current_rlp_node,
                            extension_node.clone(),
                        )))
                    }
                    TrieNode::Leaf(leaf_node) => {
                        // Do not update the nibbles left, although in this case
                        // it should be the empty slice.
                        self.current_node = None;
                        Some(Ok(TraversedNode::Leaf(current_rlp_node, leaf_node.clone())))
                    }
                }
            }
        }
    }
}

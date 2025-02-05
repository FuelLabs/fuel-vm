use core::marker::PhantomData;

use alloy_trie::{
    nodes::{
        BranchNode,
        ExtensionNode,
        LeafNode,
        RlpNode,
        TrieNode,
    },
    TrieMask,
};

use alloy_primitives::B256;
use fuel_storage::{
    Mappable,
    StorageAsMut,
    StorageMutate,
};
use nybbles::{
    self as _,
    Nibbles,
};

use alloc::sync::Arc;

use crate::common::Bytes32;

pub struct Trie<Storage, NodesTable> {
    #[allow(unused)]
    storage: Storage,
    #[allow(unused)]
    root: RlpNode,
    _phantom: PhantomData<NodesTable>,
}

impl<Storage, NodesTableType> Trie<Storage, NodesTableType> {
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
            root: RlpNode::default(),
            _phantom: PhantomData,
        }
    }
}

impl<StorageType, NodesTableType> Trie<StorageType, NodesTableType>
where
    StorageType: StorageMutate<NodesTableType, Error = anyhow::Error>,
    NodesTableType: Mappable<Key = RlpNode, Value = TrieNode, OwnedValue = TrieNode>,
{
    pub fn iter<'a>(
        &self,
        nibbles: &'a Nibbles,
    ) -> NodeIterator<'a, '_, StorageType, NodesTableType> {
        NodeIterator {
            nibbles_left: nibbles,
            current_node: Some(self.root.clone()),
            storage: &self.storage,
            _marker: PhantomData,
        }
    }

    fn store_leaf(&mut self, key: Bytes32, value: Bytes32) -> anyhow::Result<RlpNode> {
        let key_nibbles = Nibbles::unpack(key);
        // Create a new leaf node
        let leaf_node = TrieNode::Leaf(LeafNode::new(key_nibbles, value.to_vec()));
        let mut buf = Vec::with_capacity(33);
        let leaf_rlp_node: RlpNode = leaf_node.rlp(&mut buf);
        self.storage.insert(&leaf_rlp_node, &leaf_node)?;

        Ok(leaf_rlp_node)
    }

    // Helper function to create an extension node
    // pointing to a newly created node.
    fn make_linear_path_to_rlp_node(
        &mut self,
        nibbles: Nibbles,
        rlp_node: RlpNode,
    ) -> anyhow::Result<RlpNode> {
        if nibbles.as_slice().is_empty() {
            Ok(rlp_node)
        } else {
            let extension_node =
                TrieNode::Extension(ExtensionNode::new(nibbles, rlp_node.clone()));
            let mut buf = Vec::with_capacity(33);
            let extension_rlp_node: RlpNode = extension_node.rlp(&mut buf);
            self.storage.insert(&extension_rlp_node, &extension_node)?;
            Ok(extension_rlp_node)
        }
    }

    // When inserting a new leaf node, in the case we have finished traversing the trie
    // and we have nibbles left to traverse, we must create a new extension node with the
    // remaining nibbles followed by a leaf node.
    fn make_linear_path_to_leaf(
        &mut self,
        nibbles: Nibbles,
        key: Bytes32,
        value: Bytes32,
    ) -> anyhow::Result<RlpNode> {
        let leaf_rlp_node = self.store_leaf(key, value)?;

        self.make_linear_path_to_rlp_node(nibbles, leaf_rlp_node)
    }

    // If we have two nodes with a possibly common prefix, we must

    // If we reached an extension node that cannot be fully traversed (i.e. the nibbles
    // left in the path are not a prefix of the nibbles in the extension node, we
    // must:
    // 1. Create a new extension node with the common prefix between the extension node
    //    and the nibbles left,
    // 2. Create a new branch node to which the extension node created in step 1 points
    //    to,
    // 3. Create two new pairs of extension and leaf nodes, one for the key and value that
    //    we are inserting, and one for the key and value that the extension node was
    //    pointing to.

    fn branch_from_extension_node(
        &mut self,
        extension_node: ExtensionNode,
        nibbles: Nibbles,
        node_to_connect: RlpNode,
    ) -> anyhow::Result<RlpNode> {
        let common_prefix_length = extension_node.key.common_prefix_length(&nibbles);
        // If the common prefix is the same as the extension node key, then we must update
        // the child of the extension node. Because in our case the a leaf always
        // has 64 nibbles for the key, the child of the extension node cannot be a
        // branch node.
        if common_prefix_length == extension_node.key.as_slice().len() {
            // Replace the leaf node. We can use make_linear_path_to_leaf to create the
            // new leaf and extesion node, and insert them in the storage.
            // Additionally, we must remove the old extension node from the storage.
            let new_extension_rlp_node =
                self.make_linear_path_to_rlp_node(nibbles, node_to_connect)?;
            let mut buf = Vec::with_capacity(33);
            let old_extension_node = TrieNode::Extension(extension_node);
            let old_extension_rlp_node = old_extension_node.rlp(&mut buf);
            let old_extension_node_from_storage =
                self.storage.take(&old_extension_rlp_node)?;
            debug_assert_eq!(old_extension_node_from_storage, Some(old_extension_node));
            Ok(new_extension_rlp_node)
        } else {
            // The common prefix is not the same as the extension node key.
            // The extension node nibble and the input path nibble have the following
            // structure:
            // extension_node.key = [C0, ..., Ck, K0, K1, ..., Kl]
            // nibbles            = [C0, ..., Ck, N0, N1, ..., Nm]
            // In this case we can proceed as follows:
            // 1. Create a new extension Ext0 node with nibbles K1, ... , Kl, pointing to
            // the child of the previous extension node,
            // 2. Create a new extension node Ext1 with nibles N1, ... , Nm, pointing to
            // `node_to_connect`,
            // 3. Create a new branch node B for the common prefix, with two children:
            // - The first child at nibble K0 is the extension node Ext0,
            // - Then second child at nibble N0 is the extension node Ext1.
            // 4. Create an extension node with the common prefix [C0, ..., Ck],
            // pointing to the branch node B created in step 3.
            // 5. Remove the original extension node from the storage.
            let common_prefix = nibbles.slice(0..common_prefix_length);

            // 1. Create a new extension Ext0 node with nibbles K1, ... , Kl, pointing to
            // the child of the previous extension node,

            // SAFETY: This is safe because we checked that common_prefix_length is less
            // than the length of the extension node.
            let first_diverging_nibble_existing_path =
                extension_node.key[common_prefix_length];
            let other_diverging_nibbles_existing_path =
                extension_node.key.slice(common_prefix_length + 1..);
            let suffix_extension_node_existing_path_rlp = self
                .make_linear_path_to_rlp_node(
                    other_diverging_nibbles_existing_path,
                    extension_node.child.clone(),
                )?;
            // 2. Create a new extension node Ext1 with nibles N1, ... , Nm, pointing to
            // `node_to_connect`.

            // In theory it is possible that the length of the common prefix is the same
            // as the length of the input nibbles. However, this case is not possible
            // in practice, because the input nibbles are the nibbles left while
            // traversing a path from the root node to a leaf node. If the
            // extension node nibbles are longer than the input nibble, this
            // means that the trie as a logical height that is greater than
            // the length of the (logical) path from a node to the leaf. This
            // is not possible in a Merkle Patricia Trie.
            let Some(first_diverging_nibble_new_path) = nibbles.get(common_prefix_length)
            else {
                return Err(anyhow::anyhow!(
                    "Found a logical path that is longer than the trie height"
                ));
            };
            // SAFETY: We have checked that there is a nibble at index
            // common_prefix_length, hence slicing at the range below won't
            // panic
            let other_diverging_nibbles_new_path =
                nibbles.slice(common_prefix_length + 1..);

            let suffix_extension_node_new_path_rlp = self.make_linear_path_to_rlp_node(
                other_diverging_nibbles_new_path,
                node_to_connect,
            )?;

            // 3. Create a new branch node B for the common prefix
            // TODO: This is slow, we iterate through the nibble values twice
            let mut branch_node = BranchNode::default();
            self.add_child_to_branch_node(
                &mut branch_node,
                first_diverging_nibble_existing_path,
                suffix_extension_node_existing_path_rlp,
            );

            self.add_child_to_branch_node(
                &mut branch_node,
                *first_diverging_nibble_new_path,
                suffix_extension_node_new_path_rlp,
            );

            let branch_node = TrieNode::Branch(branch_node);
            let mut buf = Vec::with_capacity(33);
            let branch_node_rlp = branch_node.rlp(&mut buf);
            self.storage.insert(&branch_node_rlp, &branch_node)?;

            // 4. Create an extension node with the common prefix [C0, ..., Ck],
            // pointing to the branch node B created in step 3.
            let new_extension_node_rlp =
                self.make_linear_path_to_rlp_node(common_prefix, branch_node_rlp)?;

            // 5. Remove the original extension node from the storage.
            let mut buf = Vec::with_capacity(33);
            let old_extension_node = TrieNode::Extension(extension_node);
            let old_extension_node_rlp = old_extension_node.rlp(&mut buf);
            let old_extension_node_from_storage =
                self.storage.take(&old_extension_node_rlp)?;
            // debug_assert_eq!(old_extension_node_from_storage,
            // Some(old_extension_node));
            Ok(new_extension_node_rlp)
        }
    }

    // Utility function to expand the list of children of a BranchNode
    // Children are stored in a compact version using a bitmast. This function
    // expands the compacted list of children to an arry of 16 elements.
    fn expand_branch_node(&self, branch_node: &BranchNode) -> [Option<RlpNode>; 16] {
        let mut children = [const { None }; 16];
        for (nibble, child) in branch_node.as_ref().children() {
            children[usize::from(nibble) as usize] = child.cloned();
        }
        children
    }

    // Utility function to collapse an array of 16 children to a BranchNode.
    // This function computes the bitmask corresponding to the array of 16 children
    // for the node, and compacts the array of children.
    fn collapse_to_branch_node(&self, children: [Option<RlpNode>; 16]) -> BranchNode {
        let mut branch_node = BranchNode::default();
        let mut trie_mask = TrieMask::default();
        children.iter().enumerate().for_each(|(i, child)| {
            if let Some(child) = child {
                branch_node.stack.push(child.clone());
                trie_mask.set_bit(i as u8);
            }
        });

        branch_node
    }

    // Utility function to add a child to a branch node. This function makes
    // use of the expansion and collapse functions to update the branch node.
    fn add_child_to_branch_node(
        &self,
        branch_node: &mut BranchNode,
        nibble: u8,
        child: RlpNode,
    ) {
        let mut rlp_vector = self.expand_branch_node(branch_node);
        rlp_vector[usize::from(nibble)] = Some(child);
        let new_branch_node = self.collapse_to_branch_node(rlp_vector);
        *branch_node = new_branch_node;
    }

    // Adds a leaf to a trie.
    pub fn add_leaf(&mut self, key: Bytes32, value: Bytes32) -> anyhow::Result<RlpNode> {
        // convert the key and value to nibbles
        let key_nibbles = Nibbles::unpack(&key);

        // Now we traverse the nibble path in the trie to find the place where to
        // insert the leaf.
        // We must keep track of the nodes traversed because they will need to be updated
        // in the storage. We will insert them in a stack
        let mut node_iterator = self.iter(&key_nibbles);
        let mut nodes_in_path = Vec::new();

        // Looping instead of using for syntax or iterator transformers
        // to avoid having to give away ownership of the iterator.
        loop {
            let Some(node) = node_iterator.next() else {
                break
            };
            let node_with_next_decision = node?;
            nodes_in_path.push(node_with_next_decision);
        }

        // Check the nibbles that are left to iterate.
        let nibbles_left = node_iterator.nibbles_left();
        // We require that the tree contains at least the
        let (last_node, _decision) =
            nodes_in_path.pop().unwrap_or((TrieNode::EmptyRoot, None));
        let mut rlp_of_new_node = match last_node {
            TrieNode::EmptyRoot => {
                // If the last node is the empty root, then we can insert the leaf
                // directly. We must create a new branch node with a single child
                // at the position of the first nibble in the path.
                self.make_linear_path_to_leaf(nibbles_left, key, value)?
            }
            TrieNode::Branch(branch_node) => {
                // The last node is a branch node.
                // If there are no nibbles left, we return an error
                // Otherwise, we mus update
                // Next, we update the pointer to the child at the selected position as
                // follows: If there is no nibble left in the path (after
                // looking at the first one), then we create a leaf.
                // Otherwise, we create an extension node with the path left, pointing
                // to a new leaf node.
                let Some((first_nibble, nibbles_left)) = nibbles_left.split_first()
                else {
                    return Err(anyhow::anyhow!(
                        "The path to the leaf is already occupied by a branch node"
                    ));
                };
                let extension_node_rlp = self.make_linear_path_to_leaf(
                    Nibbles::from_nibbles(nibbles_left),
                    key,
                    value,
                )?;
                let mut new_branch_node = branch_node.clone();
                self.add_child_to_branch_node(
                    &mut new_branch_node,
                    *first_nibble,
                    extension_node_rlp,
                );
                TrieNode::Branch(new_branch_node).rlp(&mut Vec::with_capacity(33))
            }

            TrieNode::Extension(extension_node) => {
                // If the last node in the path is an extension node, then we must check
                // the common prefix between the nibbles left in the path,
                // and the nibbles referenced by the extension node.
                // Note that there must be at least one nibble on which the paths differ,
                // otherwise the iterator would have moved to the next
                // node. In this case, we create a new extension node with the common
                // prefix, pointing to a branch node. The branch node has two children:
                let leaf_rlp_node = self.store_leaf(key, value)?;
                self.branch_from_extension_node(
                    extension_node,
                    nibbles_left,
                    leaf_rlp_node,
                )?
            }
            TrieNode::Leaf(other_leaf_node) => {
                // If the last node in the path is a leaf node, then we must check
                // whether we need to update the encountered leaf node, or
                // create a new extension node with the common prefix between the
                // two leaves.

                let nibbles_left_len = nibbles_left.as_slice().len();
                let other_leaf_node_relevant_key_nibbles =
                    &other_leaf_node.key[other_leaf_node.key.len() - nibbles_left_len..];
                if other_leaf_node_relevant_key_nibbles == nibbles_left.as_slice() {
                    // We are updating the leaf that we have encountered.
                    let old_rlp_node =
                        TrieNode::Leaf(other_leaf_node).rlp(&mut Vec::with_capacity(33));
                    let rlp_node = self.store_leaf(key, value)?;
                    self.storage.remove(&old_rlp_node)?;
                    rlp_node
                } else {
                    let leaf_rlp_node = self.store_leaf(key, value)?;
                    let extension_node = ExtensionNode::new(
                        Nibbles::from_nibbles(other_leaf_node_relevant_key_nibbles),
                        leaf_rlp_node.clone(),
                    );
                    let rlp_node = self.branch_from_extension_node(
                        extension_node,
                        nibbles_left,
                        leaf_rlp_node,
                    )?;
                    let old_rlp_node =
                        TrieNode::Leaf(other_leaf_node).rlp(&mut Vec::with_capacity(33));
                    self.storage.remove(&old_rlp_node)?;
                    rlp_node
                }
            }
        };

        // We keep iterating backwards through the nodes in the path, and update with the
        // new node.
        while let Some((mut node, decision)) = nodes_in_path.pop() {
            match node {
                TrieNode::EmptyRoot => {
                    // This should not happen, either we traversed the empty root in
                    // the first iteration, or there are no nodes in the path
                    unreachable!()
                }
                TrieNode::Branch(branch_node) => {
                    let branch_node_rlp = node.rlp(&mut Vec::with_capacity(33));
                    let decision = decision.expect("Branch node must have a decision");
                    let mut new_branch_node = branch_node.clone();

                    // will update the old node with the new one
                    self.add_child_to_branch_node(
                        &mut new_branch_node,
                        decision,
                        rlp_of_new_node,
                    );
                    // store the new branch node
                    let new_node = TrieNode::Branch(new_branch_node);
                    rlp_of_new_node = new_node.rlp(&mut Vec::with_capacity(33));
                    self.storage.insert(&rlp_of_new_node, &new_node)?;
                    self.storage.remove(&branch_node_rlp)?
                }
                TrieNode::Extension(extension_node) => {
                    let extension_node_rlp = node.rlp(&mut Vec::with_capacity(33));
                    let mut new_extension_node = extension_node.clone();
                    new_extension_node.child = rlp_of_new_node.clone();
                    let new_extension_node = TrieNode::Extension(new_extension_node);
                    rlp_of_new_node = new_extension_node.rlp(&mut Vec::with_capacity(33));
                    self.storage.insert(&rlp_of_new_node, &new_extension_node)?;
                    self.storage.remove(&extension_node_rlp)?
                }
                TrieNode::Leaf(leaf_node) => {
                    // Cannot happen, we have already traversed a leaf node in the first
                    // loop of this function.
                }
            }

            // The Rlp of new node should now be the root of the trie
        }

        self.root = rlp_of_new_node;

        Ok(rlp_of_new_node)
    }
}

// Iterator for traversing a trie node with respect to a Nibble path
pub struct NodeIterator<'a, 'b, StorageType, NodesTableType> {
    nibbles_left: &'a [u8],
    current_node: Option<RlpNode>,
    storage: &'b StorageType,
    _marker: PhantomData<NodesTableType>,
}

impl<StorageType, NodesTableType> NodeIterator<'_, '_, StorageType, NodesTableType> {
    pub fn nibbles_left(&self) -> Nibbles {
        Nibbles::from_nibbles(self.nibbles_left)
    }
}

impl<StorageType, NodesTableType> Iterator
    for NodeIterator<'_, '_, StorageType, NodesTableType>
where
    StorageType: StorageMutate<NodesTableType, Error = anyhow::Error>,
    NodesTableType: Mappable<Key = RlpNode, Value = TrieNode, OwnedValue = TrieNode>,
{
    // Return the next node, and the nibble that will be used to select the next node,
    // if any.
    type Item = anyhow::Result<(TrieNode, Option<u8>)>;

    fn next(&mut self) -> Option<Self::Item> {
        let current_rlp_node = self.current_node.take()?;
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
                        // We do not update the nibbles left. This is useful
                        // when inserting a new node, as we can use the nibbles left
                        // ti identify the path to the node to be inserted.
                        self.current_node = None;
                    }
                    TrieNode::Branch(branch_node) => {
                        // Branch node: we can look at the first nibble, and
                        // select the next node based on its value.
                        let Some((next_nibble, nibbles_left)) =
                            self.nibbles_left.split_first()
                        else {
                            self.current_node = None;
                            return Some(Ok((owned_node, None)));
                        };
                        let branch_node_ref = branch_node.as_ref();
                        let next_node = branch_node_ref
                            .children()
                            .find(|(nibble, _node)| (nibble == next_nibble))
                            .unwrap()
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
                            // Do not update the nibbles left, as this information
                            // is needed when inserting a new leaf.

                            self.current_node = None;
                        }
                    }
                    TrieNode::Leaf(_leaf_node) => {
                        // Do not update the nibbles left, although in this case
                        // it should be the empty slice.
                        self.current_node = None;
                    }
                };
                Some(Ok((owned_node, self.nibbles_left.first().cloned())))
            }
        }
    }
}

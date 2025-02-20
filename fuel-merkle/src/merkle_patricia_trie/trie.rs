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

use fuel_storage::{
    Mappable,
    StorageMutate,
};
use nybbles::{
    self as _,
    Nibbles,
};

use crate::common::Bytes32;

use super::{
    apply_operations::{
        ApplyOperations,
        Pending,
    },
    nodes_iterator::{
        NodeIterator,
        TraversedNode,
    },
};

pub struct Trie<Storage, NodesTable> {
    #[allow(unused)]
    pub(crate) storage: Storage,
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

    // Returns the Rlp of the node to insert, with the set of pending changes to write to
    // the store.
    fn prepare_store_leaf(
        key: Bytes32,
        value: Bytes32,
    ) -> anyhow::Result<(RlpNode, Pending)> {
        let key_nibbles = Nibbles::unpack(key);
        // Create a new leaf node
        let leaf_node = TrieNode::Leaf(LeafNode::new(key_nibbles, value.to_vec()));
        let mut buf = Vec::with_capacity(33);
        let leaf_rlp_node: RlpNode = leaf_node.rlp(&mut buf);
        let mut pending = Pending::new();
        pending.insert(leaf_rlp_node.clone(), leaf_node);

        Ok((leaf_rlp_node, pending))
    }

    // To be called when an extension node points to a branch node with a single child.
    // TODO: Take connected node in input, remove reference to self.
    // This will allow decoupling the set of
    fn prepare_join_extension_nodes(
        extension_node: ExtensionNode,
        connected_extension_node: &ExtensionNode,
    ) -> anyhow::Result<(RlpNode, Pending)> {
        let prefix_nibbles = extension_node.as_ref().key;
        let connected_node_rlp = extension_node.clone().child;

        let suffix_nibbles = connected_extension_node.as_ref().key;
        let nibbles = prefix_nibbles.join(suffix_nibbles);
        let new_extension_node =
            ExtensionNode::new(nibbles, connected_extension_node.child.clone());

        let new_extension_node_rlp = TrieNode::Extension(new_extension_node.clone())
            .rlp(&mut Vec::with_capacity(33));
        let old_extension_node_rlp =
            TrieNode::Extension(extension_node).rlp(&mut Vec::with_capacity(33));

        let mut pending = Pending::new();
        pending.insert(
            new_extension_node_rlp.clone(),
            TrieNode::Extension(new_extension_node),
        );
        pending.delete(old_extension_node_rlp);
        pending.delete(connected_node_rlp);

        Ok((new_extension_node_rlp, pending))
    }

    fn get_child_node_from_storage(
        &self,
        branch: &BranchNode,
        nibble: u8,
    ) -> anyhow::Result<Option<(RlpNode, TrieNode)>> {
        let children = Self::expand_branch_node(branch);
        let child_rlp = children
            .get(usize::from(nibble))
            .and_then(|child| child.as_ref());

        child_rlp
            .map(|child_rlp| {
                let child_node = self
                    .storage
                    .get(&child_rlp)?
                    .ok_or_else(|| anyhow::anyhow!("Child node not found in storage"))?;
                Ok((child_rlp.clone(), child_node.into_owned()))
            })
            .transpose()
    }

    // Helper function to remove a branch node and replace it with an extension node.
    // To be used only if the branch node has one child at `nibble` position.
    fn prepare_branch_to_extension_node(
        branch_node: BranchNode,
        nibble: u8,
        node_to_connect_rlp: &RlpNode,
        node_to_connect: &TrieNode,
    ) -> anyhow::Result<(RlpNode, Pending)> {
        let branch_node_rlp =
            TrieNode::Branch(branch_node.clone()).rlp(&mut Vec::with_capacity(33));
        let expanded_branch_node = Self::expand_branch_node(&branch_node);
        debug_assert_eq!(
            expanded_branch_node
                .iter()
                .filter(|child| child.is_some())
                .count(),
            1
        );
        debug_assert!(expanded_branch_node[usize::from(nibble)].is_some());

        match node_to_connect {
            TrieNode::Branch(_branch_node) => {
                // If the node to connect is a branch node, we can create a new extension
                // node with a single nibble pointing to it.
                // (We could also have a branch node with a single child, need to check
                // what actual MPTs do in this case)
                let new_extension_node = ExtensionNode::new(
                    Nibbles::from_nibbles([nibble]),
                    node_to_connect_rlp.clone(),
                );
                let new_node = TrieNode::Extension(new_extension_node);
                let new_rlp_node = new_node.rlp(&mut Vec::with_capacity(33));
                let mut pending = Pending::new();
                pending.insert(new_rlp_node.clone(), new_node);
                pending.delete(branch_node_rlp);

                Ok((new_rlp_node, pending))
            }
            TrieNode::Extension(extension_node) => {
                // If the node to connect is an extension node, we can create a new
                // extension node with the nibbles of the extension node
                // prefixed with the nibble we are connecting
                // to, and pointing to the child of the extension node.
                let raw_nibbles: Vec<u8> = [nibble]
                    .iter()
                    .chain(extension_node.key.as_ref().iter())
                    .copied()
                    .collect();
                let new_extension_node = ExtensionNode::new(
                    Nibbles::from_nibbles(raw_nibbles),
                    extension_node.child.clone(),
                );
                let new_node = TrieNode::Extension(new_extension_node);
                let new_rlp_node = new_node.rlp(&mut Vec::with_capacity(33));
                let mut pending = Pending::new();
                pending.insert(new_rlp_node.clone(), new_node);
                pending.delete(branch_node_rlp);
                Ok((new_rlp_node, pending))
            }
            TrieNode::Leaf(_leaf_node) => {
                // If the node to connect is a leaf node, we can create a new extension
                // node with the nibble we are connecting to, pointing to the leaf node.
                // This case can happen if the branch node is at logical depth 255 in the
                // tree.
                let new_extension_node = ExtensionNode::new(
                    // Nibbles::from_nibbles(raw_nibbles),
                    Nibbles::from_nibbles(&[nibble]),
                    node_to_connect_rlp.clone(),
                );
                let new_node = TrieNode::Extension(new_extension_node);
                let new_rlp_node = new_node.rlp(&mut Vec::with_capacity(33));
                let mut pending = Pending::new();
                pending.insert(new_rlp_node.clone(), new_node);
                pending.delete(branch_node_rlp);
                Ok((new_rlp_node, pending))
            }
            TrieNode::EmptyRoot => {
                unreachable!()
            }
        }
    }

    // Helper function to create an extension node
    // pointing to a newly created node.
    fn prepare_linear_path_to_rlp_node(
        nibbles: Nibbles,
        rlp_node: RlpNode,
    ) -> anyhow::Result<(RlpNode, Pending)> {
        if nibbles.as_slice().is_empty() {
            Ok((rlp_node, Pending::new()))
        } else {
            let extension_node =
                TrieNode::Extension(ExtensionNode::new(nibbles, rlp_node.clone()));
            let mut buf = Vec::with_capacity(33);
            let extension_rlp_node: RlpNode = extension_node.rlp(&mut buf);
            let mut pending = Pending::new();
            pending.insert(extension_rlp_node.clone(), extension_node);
            Ok((extension_rlp_node, pending))
        }
    }

    // When inserting a new leaf node, in the case we have finished traversing the trie
    // and we have nibbles left to traverse, we must create a new extension node with the
    // remaining nibbles followed by a leaf node.
    fn make_linear_path_to_leaf(
        nibbles: Nibbles,
        key: Bytes32,
        value: Bytes32,
    ) -> anyhow::Result<(RlpNode, Pending)> {
        let (leaf_rlp_node, pending) = Self::prepare_store_leaf(key, value)?;

        let (extension_rlp_node, other_pending) =
            Self::prepare_linear_path_to_rlp_node(nibbles, leaf_rlp_node)?;

        Ok((extension_rlp_node, pending.merge(other_pending)))
    }

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
    fn prepare_branch_from_extension_node(
        extension_node: ExtensionNode,
        nibbles: Nibbles,
        node_to_connect: RlpNode,
    ) -> anyhow::Result<(RlpNode, Pending)> {
        let common_prefix_length = extension_node.key.common_prefix_length(&nibbles);
        // If the common prefix is the same as the extension node key, then we must update
        // the child of the extension node. Because in our case a leaf always
        // has 64 nibbles for the key, the child of the extension node cannot be a
        // branch node.
        if common_prefix_length == extension_node.key.as_slice().len() {
            // Replace the leaf node. We can use make_linear_path_to_leaf to create the
            // new leaf and extesion node, and insert them in the storage.
            // Additionally, we must remove the old extension node from the storage.
            let (new_extension_rlp_node, mut pending) =
                Self::prepare_linear_path_to_rlp_node(nibbles, node_to_connect)?;
            let mut buf = Vec::with_capacity(33);
            let old_extension_node = TrieNode::Extension(extension_node);
            let old_extension_rlp_node = old_extension_node.rlp(&mut buf);
            pending.delete(old_extension_rlp_node);

            Ok((new_extension_rlp_node, pending))
        } else {
            // The common prefix is not the same as the extension node key.
            // The extension node nibble and the input path nibble have the following
            // structure:
            // extension_node.key = [C0, ..., Ck, K0, K1, ..., Kl]
            // nibbles            = [C0, ..., Ck, N0, N1, ..., Nm]
            // In this case we can proceed as follows:
            // 1. Create a new extension Ext0 node with nibbles K1, ... , Kl, pointing to
            // the child of the previous extension node,
            // 2. Create a new extension node Ext1 with nibbles N1, ... , Nm, pointing to
            // `node_to_connect`,
            // 3. Create a new branch node B for the common prefix, with two children:
            // - The first child at nibble K0 is the extension node Ext0,
            // - Then second child at nibble N0 is the extension node Ext1.
            // 4. Create an extension node with the common prefix [C0, ..., Ck],
            // pointing to the branch node B created in step 3.
            // 5. Mark the original extension node for deletion.
            let common_prefix = nibbles.slice(0..common_prefix_length);

            // 1. Create a new extension Ext0 node with nibbles K1, ... , Kl, pointing to
            // the child of the previous extension node,

            // SAFETY: This is safe because we checked that common_prefix_length is less
            // than the length of the extension node.
            let first_diverging_nibble_existing_path =
                extension_node.key[common_prefix_length];
            let other_diverging_nibbles_existing_path =
                extension_node.key.slice(common_prefix_length + 1..);
            let (suffix_extension_node_existing_path_rlp, first_extension_node_pending) =
                Self::prepare_linear_path_to_rlp_node(
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

            let (suffix_extension_node_new_path_rlp, second_extension_node_pending) =
                Self::prepare_linear_path_to_rlp_node(
                    other_diverging_nibbles_new_path,
                    node_to_connect,
                )?;

            let mut pending =
                first_extension_node_pending.merge(second_extension_node_pending);

            // 3. Create a new branch node B for the common prefix
            // TODO: This is slow, we iterate through the nibble values twice
            let mut branch_node = BranchNode::default();
            Self::add_child_to_branch_node(
                &mut branch_node,
                first_diverging_nibble_existing_path,
                suffix_extension_node_existing_path_rlp,
            );

            Self::add_child_to_branch_node(
                &mut branch_node,
                *first_diverging_nibble_new_path,
                suffix_extension_node_new_path_rlp,
            );

            let branch_node = TrieNode::Branch(branch_node);
            let mut buf = Vec::with_capacity(33);
            let branch_node_rlp = branch_node.rlp(&mut buf);

            pending.insert(branch_node_rlp.clone(), branch_node);

            // 4. Create an extension node with the common prefix [C0, ..., Ck],
            // pointing to the branch node B created in step 3.
            let (new_extension_node_rlp, new_extension_node_pending) =
                Self::prepare_linear_path_to_rlp_node(common_prefix, branch_node_rlp)?;

            let mut pending = pending.merge(new_extension_node_pending);

            // 5. Mark the original extension node from the storage.
            let mut buf = Vec::with_capacity(33);
            let old_extension_node = TrieNode::Extension(extension_node);
            let old_extension_node_rlp = old_extension_node.rlp(&mut buf);
            pending.delete(old_extension_node_rlp);
            Ok((new_extension_node_rlp, pending))
        }
    }

    // Utility function to expand the list of children of a BranchNode
    // Children are stored in a compact version using a bitmast. This function
    // expands the compacted list of children to an arry of 16 elements.
    fn expand_branch_node(branch_node: &BranchNode) -> [Option<RlpNode>; 16] {
        let mut children = [const { None }; 16];
        for (nibble, child) in branch_node.as_ref().children() {
            children[usize::from(nibble) as usize] = child.cloned();
        }
        children
    }

    // Utility function to collapse an array of 16 children to a BranchNode.
    // This function computes the bitmask corresponding to the array of 16 children
    // for the node, and compacts the array of children.
    fn collapse_to_branch_node(children: [Option<RlpNode>; 16]) -> BranchNode {
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

    fn delete_child_from_branch_node(
        branch_node: &mut BranchNode,
        nibble: u8,
    ) -> RlpNode {
        debug_assert!(branch_node.as_ref().state_mask.count_ones() > 1);
        let mut rlp_vector = Self::expand_branch_node(branch_node);
        rlp_vector[usize::from(nibble)] = None;
        let new_branch_node = Self::collapse_to_branch_node(rlp_vector);
        *branch_node = new_branch_node;
        TrieNode::Branch(branch_node.clone()).rlp(&mut Vec::with_capacity(33))
    }

    fn prepare_delete_child_from_branch_node(
        branch_node_rlp: RlpNode,
        branch_node: &mut BranchNode,
        nibble: u8,
    ) -> (RlpNode, Pending) {
        let new_branch_node_rlp =
            Self::delete_child_from_branch_node(branch_node, nibble);
        let mut pending = Pending::new();
        pending.insert(
            new_branch_node_rlp.clone(),
            TrieNode::Branch(branch_node.clone()),
        );
        pending.delete(branch_node_rlp);
        (new_branch_node_rlp, pending)
    }

    // Utility function to add a child to a branch node. This function makes
    // use of the expansion and collapse functions to update the branch node.
    fn add_child_to_branch_node(
        branch_node: &mut BranchNode,
        nibble: u8,
        child: RlpNode,
    ) -> RlpNode {
        let mut rlp_vector = Self::expand_branch_node(branch_node);
        rlp_vector[usize::from(nibble)] = Some(child);
        let new_branch_node = Self::collapse_to_branch_node(rlp_vector);
        *branch_node = new_branch_node;
        TrieNode::Branch(branch_node.clone()).rlp(&mut Vec::with_capacity(33))
    }

    fn prepare_add_child_to_branch_node(
        branch_node_rlp: RlpNode,
        branch_node: &mut BranchNode,
        nibble: u8,
        child: RlpNode,
    ) -> (RlpNode, Pending) {
        let new_branch_node_rlp =
            Self::add_child_to_branch_node(branch_node, nibble, child);
        let mut pending = Pending::new();
        pending.insert(
            new_branch_node_rlp.clone(),
            TrieNode::Branch(branch_node.clone()),
        );
        pending.delete(branch_node_rlp);
        (new_branch_node_rlp, pending)
    }

    fn prepare_update_extension_node(
        extension_node_rlp: RlpNode,
        mut extension_node: ExtensionNode,
        new_child_rlp: RlpNode,
    ) -> (RlpNode, Pending) {
        let mut pending = Pending::new();
        extension_node.child = new_child_rlp;
        let new_extension_node_rlp =
            TrieNode::Extension(extension_node.clone()).rlp(&mut Vec::with_capacity(33));
        pending.insert(
            new_extension_node_rlp.clone(),
            TrieNode::Extension(extension_node),
        );
        pending.delete(extension_node_rlp);
        (new_extension_node_rlp, pending)
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
        let last_traversed_node =
            nodes_in_path.pop().unwrap_or(TraversedNode::EmptyRoot(
                TrieNode::EmptyRoot.rlp(&mut Vec::with_capacity(33)),
            ));
        // We have traversed the path in the tree. The new leaf will be
        // appended to the last node in the path. Different cases should
        // be considered according to the type of the last traversed node.
        let (mut rlp_of_new_node, pending_changes) = match last_traversed_node {
            TraversedNode::EmptyRoot(_node_rlp) => {
                // If the last traversed node is the empty root, then we can insert the
                // leaf directly. We must create a new branch node with a
                // single child at the position of the first nibble in the
                // path. Note that the empty root is not removed from the
                // storage, as we might have other Merkle Trees pointing
                // at it as their root.
                Self::make_linear_path_to_leaf(nibbles_left, key, value)?
            }
            TraversedNode::Branch(branch_node_rlp, branch_node, decision) => {
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
                // Check that the decision made by the iterator is indeed consistent with
                // the first nibble in the reamining path.
                debug_assert_eq!(decision, *first_nibble);
                // Make a linear path to the leaf node. This could be either a
                // leaf node, if the nibbles left are empty, or an extension node
                // pointing to the leaf node.
                let (extension_or_leaf_node_rlp, pending) =
                    Self::make_linear_path_to_leaf(
                        Nibbles::from_nibbles(nibbles_left),
                        key,
                        value,
                    )?;
                let mut new_branch_node = branch_node.clone();
                let (new_branch_node_rlp, other_pending) =
                    Self::prepare_add_child_to_branch_node(
                        branch_node_rlp,
                        &mut new_branch_node,
                        *first_nibble,
                        extension_or_leaf_node_rlp,
                    );
                (new_branch_node_rlp, pending.merge(other_pending))
            }

            TraversedNode::Extension(_extension_node_rlp, extension_node) => {
                // If the last node in the path is an extension node, then we must check
                // the common prefix between the nibbles left in the path,
                // and the nibbles referenced by the extension node.
                // Note that there must be at least one nibble on which the paths differ,
                // otherwise the iterator would have moved to the next
                // node. In this case, we create a new extension node with the common
                // prefix, pointing to a branch node. The branch node has two children:
                let (leaf_rlp_node, pending) = Self::prepare_store_leaf(key, value)?;
                let (branch_rlp_node, other_pending) =
                    Self::prepare_branch_from_extension_node(
                        extension_node,
                        nibbles_left,
                        leaf_rlp_node,
                    )?;
                (branch_rlp_node, pending.merge(other_pending))
            }
            TraversedNode::Leaf(_other_leaf_node_rlp, other_leaf_node) => {
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
                    let (rlp_node, mut pending) = Self::prepare_store_leaf(key, value)?;
                    pending.delete(old_rlp_node);
                    (rlp_node, pending)
                } else {
                    let (leaf_rlp_node, pending) = Self::prepare_store_leaf(key, value)?;
                    let extension_node = ExtensionNode::new(
                        Nibbles::from_nibbles(other_leaf_node_relevant_key_nibbles),
                        leaf_rlp_node.clone(),
                    );
                    let (rlp_node, other_pending) =
                        Self::prepare_branch_from_extension_node(
                            extension_node,
                            nibbles_left,
                            leaf_rlp_node,
                        )?;
                    let mut pending = pending.merge(other_pending);
                    let old_rlp_node =
                        TrieNode::Leaf(other_leaf_node).rlp(&mut Vec::with_capacity(33));
                    pending.delete(old_rlp_node);
                    (rlp_node, pending)
                }
            }
        };

        // Apply the pending changes to insert the leaf to the storage
        self.apply_operations(pending_changes)?;

        // We keep iterating backwards through the nodes in the path, and update with the
        // new node.
        while let Some(traversed_node) = nodes_in_path.pop() {
            match traversed_node {
                TraversedNode::EmptyRoot(_) => {
                    // This should not happen, either we traversed the empty root in
                    // the first iteration, or there are no nodes in the path
                    unreachable!()
                }
                TraversedNode::Branch(branch_node_rlp, branch_node, decision) => {
                    let mut new_branch_node = branch_node.clone();

                    // will update the old node with the new one
                    let (rlp, pending) = Self::prepare_add_child_to_branch_node(
                        branch_node_rlp,
                        &mut new_branch_node,
                        decision,
                        rlp_of_new_node.clone(),
                    );

                    self.apply_operations(pending)?;

                    rlp_of_new_node = rlp;
                }
                TraversedNode::Extension(extension_node_rlp, extension_node) => {
                    let (rlp, pending) = Self::prepare_update_extension_node(
                        extension_node_rlp,
                        extension_node,
                        rlp_of_new_node.clone(),
                    );

                    self.apply_operations(pending)?;
                    rlp_of_new_node = rlp;
                }
                TraversedNode::Leaf(_leaf_node_rlp, _leaf_node) => {
                    // Cannot happen, we have already traversed a leaf node in the first
                    // loop of this function.
                    unreachable!()
                }
            }

            // The Rlp of new node should now be the root of the trie
        }

        self.root = rlp_of_new_node.clone();

        Ok(rlp_of_new_node)
    }

    pub fn delete_leaf(&mut self, key: &Nibbles) -> anyhow::Result<RlpNode> {
        // The deletion process is performed in 4 different stages
        enum Stage {
            // At PreDeletion stage we check if there is a leaf at the key given in
            // input, eventually removing it from the db and moving to the
            // in-progress stage
            PreDeletion,
            // When deletion is in progress, we traverse the path to the leaf deleted in
            // reverse order:
            // * any extension node (at most 1) in this stage is removed,
            // * branch nodes with 2 children are updated to point to the remaining
            //   child,
            // and converted to an extension node. In this case we proceed to the
            // JoinExtensionNodes stage
            // * branch nodes with more than 2 children are updated to remove the child
            //   at the
            // nibble corresponding to the decision taken when traversing the path. In
            // this case we can skip the JoinExtensionNodes stage and
            // proceed to the PostDeletion stage.
            InProgress,
            // In the JoinExtensionNodes stage, we check whether the next node in the
            // reverse path is an extension node, and join it with the
            // current node if it is the case. Otherwise, we move to the
            // PostDeletion stage
            JoinExtensionNodes(RlpNode),
            // In the PostDeletion stage, we simply update the nodes in the path to
            // reflect the changes made in the previous stages.
            PostDeletion(RlpNode),
        }

        if key.len() != 64 {
            return Err(anyhow::anyhow!("Key must have 64 nibbles"));
        }

        // Traverse the path to the leaf node to be deleted

        let mut node_iterator = self.iter(key);
        let mut nodes_in_path = Vec::new();
        loop {
            let Some(node) = node_iterator.next() else {
                break
            };
            let node_with_next_decision = node?;
            nodes_in_path.push(node_with_next_decision);
        }

        let mut stage = Stage::PreDeletion;
        while let Some(traversed_node) = nodes_in_path.pop() {
            // At this stage we remove a leaf node from the trie if one with the given key
            // is found. Otherwsie, we leave the tree unchanged and complete
            // the deletion process.
            match stage {
                Stage::PreDeletion => match traversed_node {
                    TraversedNode::EmptyRoot(_)
                    | TraversedNode::Branch(_, _, _)
                    | TraversedNode::Extension(_, _) => return Ok(self.root.clone()),
                    TraversedNode::Leaf(ref leaf_node_rlp, ref _leaf_node) => {
                        self.storage.remove(leaf_node_rlp)?;
                        stage = Stage::InProgress;
                    }
                },
                Stage::InProgress => {
                    match traversed_node {
                        TraversedNode::EmptyRoot(_) | TraversedNode::Leaf(_, _) => {
                            // Cannot happen, we have traversed a leaf already
                            anyhow::bail!("Empty root node in the path")
                        }
                        TraversedNode::Extension(
                            ref extension_node_rlp,
                            _extension_node,
                        ) => {
                            // Remove the extension node
                            self.storage.remove(extension_node_rlp)?;
                            stage = Stage::InProgress;
                        }
                        TraversedNode::Branch(
                            ref branch_node_rlp,
                            branch_node,
                            decision,
                        ) => {
                            let branch_node_ref = branch_node.as_ref();
                            let siblings_of_node_being_deleted = branch_node_ref
                                .children()
                                .filter(|(nibble, node)| {
                                    nibble != &decision && node.is_some()
                                })
                                .collect::<Vec<_>>();
                            let Some((first_sibling, other_siblings)) =
                                siblings_of_node_being_deleted.split_first()
                            else {
                                anyhow::bail!("Branch node with a single child");
                            };
                            let should_transform_into_extension_node =
                                other_siblings.is_empty();

                            if should_transform_into_extension_node {
                                let (nibble, _node) = first_sibling;
                                let Some((child_at_nibble_rlp, child_at_nibble)) = self
                                    .get_child_node_from_storage(
                                    &branch_node,
                                    *nibble,
                                )?
                                else {
                                    anyhow::bail!("Child node not found in storage")
                                };

                                let (new_extension_node_rlp, pending) =
                                    Self::prepare_branch_to_extension_node(
                                        branch_node.clone(),
                                        *nibble,
                                        &child_at_nibble_rlp,
                                        &child_at_nibble,
                                    )?;
                                self.apply_operations(pending)?;
                                stage = Stage::JoinExtensionNodes(new_extension_node_rlp);
                            } else {
                                let mut new_branch_node = branch_node.clone();
                                let (new_branch_node_rlp, pending) =
                                    Self::prepare_delete_child_from_branch_node(
                                        branch_node_rlp.clone(),
                                        &mut new_branch_node,
                                        decision,
                                    );
                                self.apply_operations(pending)?;
                                stage = Stage::PostDeletion(new_branch_node_rlp);
                            }
                        }
                    }
                }
                Stage::JoinExtensionNodes(rlp_node) => {
                    match traversed_node {
                        TraversedNode::EmptyRoot(_) | TraversedNode::Leaf(_, _) => {
                            // Cannot happen, we have traversed a leaf already
                            anyhow::bail!("Empty root node in the path")
                        }
                        TraversedNode::Branch(
                            ref branch_node_rlp,
                            branch_node,
                            decision,
                        ) => {
                            // simply update the branch node to point to the new extension
                            // node, move to the PostDeletion
                            // stage
                            let mut new_branch_node = branch_node.clone();
                            let (new_branch_node_rlp, pending) =
                                Self::prepare_add_child_to_branch_node(
                                    branch_node_rlp.clone(),
                                    &mut new_branch_node,
                                    decision,
                                    rlp_node,
                                );
                            self.apply_operations(pending)?;

                            stage = Stage::PostDeletion(new_branch_node_rlp);
                        }
                        TraversedNode::Extension(_extension_node_rlp, extension_node) => {
                            let connected_node = self.storage.get(&extension_node.child)?.ok_or_else(|| {
                                anyhow::anyhow!("Node referenced by extension node not found in storage")
                            })?;
                            let TrieNode::Extension(ref connected_node) =
                                connected_node.as_ref()
                            else {
                                return Err(anyhow::anyhow!(
                                    "Extension node must point to another extension node"
                                ));
                            };
                            // We can be in this case only if the current rlpNode refers
                            // to an extension node. In this
                            // case we join the two extension nodes,
                            let (new_extension_node_rlp, pending) =
                                Self::prepare_join_extension_nodes(
                                    extension_node,
                                    connected_node,
                                )?;
                            self.apply_operations(pending)?;

                            stage = Stage::PostDeletion(new_extension_node_rlp);
                        }
                    }
                }
                Stage::PostDeletion(rlp_node) => {
                    match traversed_node {
                        TraversedNode::EmptyRoot(_) | TraversedNode::Leaf(_, _) => {
                            // Cannot happen, we have traversed a leaf already
                            anyhow::bail!("Empty root node in the path")
                        }
                        TraversedNode::Branch(
                            ref branch_node_rlp,
                            branch_node,
                            decision,
                        ) => {
                            let mut new_branch_node = branch_node.clone();
                            let (new_branch_node_rlp, pending) =
                                Self::prepare_add_child_to_branch_node(
                                    branch_node_rlp.clone(),
                                    &mut new_branch_node,
                                    decision,
                                    rlp_node,
                                );
                            self.apply_operations(pending)?;
                            stage = Stage::PostDeletion(new_branch_node_rlp);
                        }
                        TraversedNode::Extension(
                            ref extension_node_rlp,
                            extension_node,
                        ) => {
                            let (new_extension_node_rlp, pending) =
                                Self::prepare_update_extension_node(
                                    extension_node_rlp.clone(),
                                    extension_node,
                                    rlp_node.clone(),
                                );
                            self.apply_operations(pending)?;
                            stage = Stage::PostDeletion(new_extension_node_rlp);
                        }
                    }
                }
            }
        }

        match stage {
            Stage::PreDeletion => {
                anyhow::bail!("Can't be in predeletion stage.")
            }
            Stage::InProgress => {
                // We traversed all the nodes in the path, keeping deleting nodes.
                // The tree is now empty.
                self.storage.insert(
                    &TrieNode::EmptyRoot.rlp(&mut Vec::with_capacity(33)),
                    &TrieNode::EmptyRoot,
                )?;
                self.root = TrieNode::EmptyRoot.rlp(&mut Vec::with_capacity(33));
            }
            Stage::PostDeletion(rlp_node) | Stage::JoinExtensionNodes(rlp_node) => {
                self.root = rlp_node;
            }
        }
        Ok(self.root.clone())
    }
}

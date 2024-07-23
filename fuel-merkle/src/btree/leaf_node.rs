use crate::btree::{
    find_index_of_insertion,
    HashedValue,
    Key,
};
use alloc::vec::Vec;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LeafSubNode {
    pub key: Key,
    pub value: HashedValue,
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StorageLeafNode {
    pub sub_nodes: Vec<LeafSubNode>,
}

/// The leaf node of the B-Tree that guarantees that the number of keys is in the range
/// `[N/2, N]`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LeafNode<const N: u8> {
    /// The key of the node is one of the keys from `keys_and_values`.
    node_key: Key,
    sub_nodes: Vec<LeafSubNode>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LeafInsertionResult<const N: u8> {
    Added(LeafNode<N>),
    Updated(LeafNode<N>),
    Overflowed {
        new_left_leaf: LeafNode<N>,
        orphan_sub_node: LeafSubNode,
        new_right_leaf: LeafNode<N>,
    },
}

#[cfg(feature = "test-helpers")]
impl<const N: u8> Default for LeafNode<N> {
    fn default() -> Self {
        Self::new_root(Default::default())
    }
}

impl<const N: u8> LeafNode<N> {
    pub const MINIMUM_KEYS: u8 = {
        if N % 2 == 1 {
            panic!("`N` must be an even number");
        }
        if N < 2 {
            panic!("`N` more than `1`");
        }
        N / 2
    };
    pub const RIGHT_HALF_STAR_INDEX: u8 = Self::MINIMUM_KEYS + 1;

    pub fn new_root(sub_node: LeafSubNode) -> Self {
        let is_root = true;
        let root_key = sub_node.key;
        let mut sub_nodes = Vec::with_capacity(N as usize);
        sub_nodes.push(sub_node);
        Self::new_inner(is_root, root_key, sub_nodes)
            .expect("The number of keys in allowed range")
    }

    pub fn from_storage(node_key: Key, sub_nodes: Vec<LeafSubNode>) -> Option<Self> {
        let is_root = false;
        Self::new_inner(is_root, node_key, sub_nodes)
    }

    pub fn new_child(sub_nodes: Vec<LeafSubNode>) -> Option<Self> {
        if sub_nodes.is_empty() {
            return None
        }

        let is_root = false;
        let node_key = sub_nodes[0].key;
        Self::new_inner(is_root, node_key, sub_nodes)
    }

    fn new_inner(
        is_root: bool,
        node_key: Key,
        keys_and_values: Vec<LeafSubNode>,
    ) -> Option<Self> {
        let more_keys = keys_and_values.len() > N as usize;
        let less_keys = keys_and_values.len() < Self::MINIMUM_KEYS as usize;
        if more_keys || !is_root && less_keys {
            None
        } else {
            Some(Self {
                node_key,
                sub_nodes: keys_and_values,
            })
        }
    }

    pub fn node_key(&self) -> &Key {
        &self.node_key
    }

    pub fn sub_nodes(&self) -> &Vec<LeafSubNode> {
        &self.sub_nodes
    }

    pub fn into_storage_node(self) -> StorageLeafNode {
        StorageLeafNode {
            sub_nodes: self.sub_nodes,
        }
    }

    // TODO: Consider other variants of hash calculation that use
    //  binary Merkle Tree to minimize the proof size.
    pub fn hash(&self) -> HashedValue {
        use digest::Digest;
        let mut hash = sha2::Sha256::new();
        for sub_node in self.sub_nodes.iter() {
            hash.update(sub_node.key);
            hash.update(sub_node.value);
        }
        hash.finalize().into()
    }

    pub fn insert_leaf(mut self, new_leaf: LeafSubNode) -> LeafInsertionResult<N> {
        let index = find_index_of_insertion(&self.sub_nodes, &new_leaf.key);

        match index {
            Ok(index) => {
                self.sub_nodes[index] = new_leaf;
                LeafInsertionResult::Updated(self)
            }
            Err(index) => {
                self.sub_nodes.insert(index, new_leaf);
                if self.sub_nodes.len() > N as usize {
                    let (left, mid, right) = self.split();
                    LeafInsertionResult::Overflowed {
                        new_left_leaf: left,
                        orphan_sub_node: mid,
                        new_right_leaf: right,
                    }
                } else {
                    LeafInsertionResult::Added(self)
                }
            }
        }
    }

    fn split(self) -> (LeafNode<N>, LeafSubNode, LeafNode<N>) {
        assert_eq!(self.sub_nodes.len(), N.saturating_add(1) as usize);

        let mid_pair = self.sub_nodes[Self::MINIMUM_KEYS as usize];
        let mut right_sub_nodes = Vec::with_capacity(N as usize);
        right_sub_nodes
            .extend_from_slice(&self.sub_nodes[Self::RIGHT_HALF_STAR_INDEX as usize..]);
        let mut left_sub_nodes = self.sub_nodes;
        left_sub_nodes.truncate(Self::MINIMUM_KEYS as usize);
        debug_assert_eq!(left_sub_nodes.len(), Self::MINIMUM_KEYS as usize);
        debug_assert_eq!(right_sub_nodes.len(), Self::MINIMUM_KEYS as usize);

        let left =
            Self::new_child(left_sub_nodes).expect("The number of keys in allowed range");
        let right = Self::new_child(right_sub_nodes)
            .expect("The number of keys in allowed range");

        (left, mid_pair, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(key: u8, value: u8) -> LeafSubNode {
        LeafSubNode {
            key: [key; 32],
            value: [value; 32],
        }
    }

    #[test]
    fn root_node_can_be_created_with_1_leaf() {
        let _ = LeafNode::<2>::new_root(leaf(1, 1));
    }

    #[test]
    fn child_node_can_be_created_with_2_leafs() {
        // When
        let result = LeafNode::<2>::new_child(vec![leaf(1, 1), leaf(2, 2)]);

        // Then
        let _node = result.expect("Should be able to create a node with 2 leafs");
    }

    #[test]
    fn child_node_creation_should_fails_with_3_leafs() {
        // When
        let result = LeafNode::<2>::new_child(vec![leaf(1, 1), leaf(2, 2), leaf(3, 3)]);

        // Then
        assert!(result.is_none());
    }

    #[test]
    fn insert_new_value_to_root_node_adds_it() {
        // Given
        let node = LeafNode::<2>::new_root(leaf(1, 1));

        // When
        let result = node.insert_leaf(leaf(2, 2));

        // Then
        let expected = LeafNode::<2>::new_child(vec![leaf(1, 1), leaf(2, 2)]).unwrap();
        assert_eq!(result, LeafInsertionResult::Added(expected));
    }

    #[test]
    fn update_new_value_to_root_node_updates_it() {
        // Given
        let node = LeafNode::<2>::new_root(leaf(1, 1));

        // When
        let result = node.insert_leaf(leaf(1, 2));

        // Then
        let expected = LeafNode::<2>::new_root(leaf(1, 2));
        assert_eq!(result, LeafInsertionResult::Updated(expected));
    }

    #[test]
    fn insert_new_value_to_full_child_node_2_splits_it() {
        // Given
        let node = LeafNode::<2>::new_child(vec![leaf(1, 1), leaf(2, 2)]).unwrap();

        // When
        let result = node.insert_leaf(leaf(3, 3));

        // Then
        let expected_left = LeafNode::<2>::new_child(vec![leaf(1, 1)]).unwrap();
        let expected_mid = leaf(2, 2);
        let expected_right = LeafNode::<2>::new_child(vec![leaf(3, 3)]).unwrap();
        assert_eq!(
            result,
            LeafInsertionResult::Overflowed {
                new_left_leaf: expected_left,
                orphan_sub_node: expected_mid,
                new_right_leaf: expected_right,
            }
        );
    }

    #[test]
    fn insert_new_value_to_full_child_node_4_splits_it() {
        // Given
        let node = LeafNode::<4>::new_child(vec![
            leaf(1, 1),
            leaf(2, 2),
            leaf(3, 3),
            leaf(4, 4),
        ])
        .unwrap();

        // When
        let result = node.insert_leaf(leaf(5, 5));

        // Then
        let expected_left =
            LeafNode::<4>::new_child(vec![leaf(1, 1), leaf(2, 2)]).unwrap();
        let expected_mid = leaf(3, 3);
        let expected_right =
            LeafNode::<4>::new_child(vec![leaf(4, 4), leaf(5, 5)]).unwrap();
        assert_eq!(
            result,
            LeafInsertionResult::Overflowed {
                new_left_leaf: expected_left,
                orphan_sub_node: expected_mid,
                new_right_leaf: expected_right,
            }
        );
    }

    #[test]
    fn insert_2_values_to_root_node_splits_it_at_the_end() {
        // Given
        let node = LeafNode::<2>::new_root(leaf(3, 3));

        // When
        let LeafInsertionResult::Added(node) = node.insert_leaf(leaf(2, 2)) else {
            panic!("Should be able to add the second key");
        };
        let result = node.insert_leaf(leaf(1, 1));

        // Then
        let expected_left = LeafNode::<2>::new_child(vec![leaf(1, 1)]).unwrap();
        let expected_mid = leaf(2, 2);
        let expected_right = LeafNode::<2>::new_child(vec![leaf(3, 3)]).unwrap();
        assert_eq!(
            result,
            LeafInsertionResult::Overflowed {
                new_left_leaf: expected_left,
                orphan_sub_node: expected_mid,
                new_right_leaf: expected_right,
            }
        );
    }
}

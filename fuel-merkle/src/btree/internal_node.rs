use crate::btree::{
    find_index_of_insertion,
    leaf_node::LeafSubNode,
    ChildHash,
    ChildKey,
    HashedValue,
    Key,
};
use alloc::vec::Vec;
use digest::Digest;
use fuel_storage::MerkleRoot;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Edge {
    pub key: ChildKey,
    pub hash: ChildHash,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StorageInternalNode {
    pub sub_nodes_root: Option<MerkleRoot>,
    pub edges_root: Option<MerkleRoot>,
    pub sub_nodes: Vec<LeafSubNode>,
    /// The edges to the children of the node.
    /// The number of edges is always one more than the number of sub-nodes.
    pub edges_to_children: Vec<Edge>,
}

/// The internal node of the B-Tree that guarantees that the number of keys is in the
/// range `[N/2, N]`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InternalNode<const N: u8> {
    /// The key of the node is one of the keys in the sub-nodes.
    node_key: Key,
    sub_nodes_root: Option<MerkleRoot>,
    edges_root: Option<MerkleRoot>,
    sub_nodes: Vec<LeafSubNode>,
    /// The edges to the children of the node.
    /// The number of edges is always one more than the number of sub-nodes.
    edges_to_children: Vec<Edge>,
}

#[derive(Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum InternalInsertionResult<const N: u8> {
    Added {
        updated_node: InternalNode<N>,
    },
    Updated(InternalNode<N>),
    Overflowed {
        new_left_node: InternalNode<N>,
        orphan_sub_node: LeafSubNode,
        new_right_node: InternalNode<N>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum LookupResult<'a> {
    Found(&'a mut LeafSubNode),
    NoFound(&'a mut Edge),
}

impl<const N: u8> InternalNode<N> {
    pub const MAXIMUM_NUMBER_OF_EDGES: u8 = N + 1;
    pub const MINIMUM_KEYS: u8 = {
        if N % 2 == 1 {
            panic!("`N` must be an even number");
        }
        if N < 2 {
            panic!("`N` more than `1`");
        }
        N / 2
    };
    pub const NUMBER_OF_EDGES_DURING_SPLIT: u8 = Self::MINIMUM_KEYS + 1;
    pub const RIGHT_HALF_SUB_NODES_STAR_INDEX: u8 = Self::MINIMUM_KEYS + 1;

    pub fn new_root(left_edge: Edge, sub_node: LeafSubNode, right_edge: Edge) -> Self {
        let is_root = true;
        let root_key = sub_node.key;
        let mut sub_nodes = Vec::with_capacity(N as usize);
        sub_nodes.push(sub_node);

        let mut edges = Vec::with_capacity(Self::MAXIMUM_NUMBER_OF_EDGES as usize);
        edges.push(left_edge);
        edges.push(right_edge);
        Self::new_inner(is_root, root_key, None, None, sub_nodes, edges)
            .expect("The number of keys in allowed range")
    }

    pub fn from_storage(node_key: Key, storage: StorageInternalNode) -> Option<Self> {
        let is_root = false;
        Self::new_inner(
            is_root,
            node_key,
            storage.sub_nodes_root,
            storage.edges_root,
            storage.sub_nodes,
            storage.edges_to_children,
        )
    }

    pub fn new_child(sub_nodes: Vec<LeafSubNode>, edges: Vec<Edge>) -> Option<Self> {
        if sub_nodes.is_empty() {
            return None
        }

        let is_root = false;
        let node_key = sub_nodes[0].key;
        Self::new_inner(is_root, node_key, None, None, sub_nodes, edges)
    }

    fn new_inner(
        is_root: bool,
        node_key: Key,
        sub_nodes_hash: Option<MerkleRoot>,
        edges_hash: Option<MerkleRoot>,
        sub_nodes: Vec<LeafSubNode>,
        edges: Vec<Edge>,
    ) -> Option<Self> {
        let more_keys = sub_nodes.len() > N as usize;
        let less_keys = sub_nodes.len() < Self::MINIMUM_KEYS as usize;
        let sub_nodes_count = sub_nodes.len();
        let expected_edges_count = sub_nodes_count.saturating_add(1);
        let actual_edges_count = edges.len();
        if expected_edges_count != actual_edges_count
            || more_keys
            || !is_root && less_keys
        {
            None
        } else {
            let node = Self {
                node_key,
                sub_nodes_root: sub_nodes_hash,
                edges_root: edges_hash,
                sub_nodes,
                edges_to_children: edges,
            };
            Some(node)
        }
    }

    pub fn node_key(&self) -> &Key {
        &self.node_key
    }

    pub fn sub_nodes(&self) -> &Vec<LeafSubNode> {
        &self.sub_nodes
    }

    pub fn edges(&self) -> &Vec<Edge> {
        &self.edges_to_children
    }

    pub fn into_storage_node(self) -> StorageInternalNode {
        StorageInternalNode {
            sub_nodes_root: self.sub_nodes_root,
            edges_root: self.edges_root,
            sub_nodes: self.sub_nodes,
            edges_to_children: self.edges_to_children,
        }
    }

    // TODO: Consider other variants of hash calculation that use
    //  binary Merkle Tree to minimize the proof size.
    pub fn hash(&mut self) -> HashedValue {
        let mut hash = sha2::Sha256::new();

        if self.sub_nodes_root.is_none() {
            self.cache_sub_nodes_root();
        }

        if self.edges_root.is_none() {
            self.cache_edges_root();
        }

        let Some(sub_nodes_root) = &self.sub_nodes_root else {
            unreachable!("We've called `cache_sub_nodes_root` above")
        };

        let Some(edges_root) = &self.edges_root else {
            unreachable!("We've called `cache_edges_root` above")
        };

        hash.update(sub_nodes_root);
        hash.update(edges_root);

        hash.finalize().into()
    }

    pub fn cache_sub_nodes_root(&mut self) {
        let mut hash = sha2::Sha256::new();

        for sub_node in self.sub_nodes.iter() {
            hash.update(sub_node.key);
            hash.update(sub_node.value);
        }

        self.sub_nodes_root = Some(hash.finalize().into());
    }

    pub fn cache_edges_root(&mut self) {
        let mut hash = sha2::Sha256::new();

        for edge in self.edges_to_children.iter() {
            hash.update(edge.hash);
        }

        self.edges_root = Some(hash.finalize().into());
    }

    pub fn lookup(&mut self, key: &Key) -> LookupResult {
        let index = find_index_of_insertion(&self.sub_nodes, key);
        match index {
            Ok(index) => {
                self.sub_nodes_root = None;
                LookupResult::Found(&mut self.sub_nodes[index])
            }
            Err(index) => {
                self.edges_root = None;
                LookupResult::NoFound(&mut self.edges_to_children[index])
            }
        }
    }

    pub fn insert_edge(
        mut self,
        left_edge: Edge,
        sub_node: LeafSubNode,
        right_edge: Edge,
    ) -> InternalInsertionResult<N> {
        self.sub_nodes_root = None;
        self.edges_root = None;
        let index = find_index_of_insertion(&self.sub_nodes, &sub_node.key);

        match index {
            Ok(index) => {
                self.sub_nodes[index] = sub_node;
                let left_edge_index = index;
                let right_edge_index = index.saturating_add(1);
                self.edges_to_children[left_edge_index] = left_edge;
                self.edges_to_children[right_edge_index] = right_edge;

                InternalInsertionResult::Updated(self)
            }
            Err(index) => {
                self.sub_nodes.insert(index, sub_node);
                let left_edge_index = index;
                let right_edge_index = index.saturating_add(1);
                self.edges_to_children[left_edge_index] = left_edge;
                self.edges_to_children.insert(right_edge_index, right_edge);

                if self.sub_nodes.len() > N as usize {
                    let (left, mid, right) = self.split();

                    InternalInsertionResult::Overflowed {
                        new_left_node: left,
                        orphan_sub_node: mid,
                        new_right_node: right,
                    }
                } else {
                    InternalInsertionResult::Added { updated_node: self }
                }
            }
        }
    }

    fn split(self) -> (InternalNode<N>, LeafSubNode, InternalNode<N>) {
        assert_eq!(self.sub_nodes.len(), N.saturating_add(1) as usize);

        let mid_pair = self.sub_nodes[Self::MINIMUM_KEYS as usize];
        let mut right_sub_nodes = Vec::with_capacity(N as usize);
        right_sub_nodes.extend_from_slice(
            &self.sub_nodes[Self::RIGHT_HALF_SUB_NODES_STAR_INDEX as usize..],
        );
        let mut left_sub_nodes = self.sub_nodes;
        left_sub_nodes.truncate(Self::MINIMUM_KEYS as usize);
        debug_assert_eq!(left_sub_nodes.len(), Self::MINIMUM_KEYS as usize);
        debug_assert_eq!(right_sub_nodes.len(), Self::MINIMUM_KEYS as usize);

        let mut right_edges = Vec::with_capacity(Self::MAXIMUM_NUMBER_OF_EDGES as usize);
        right_edges.extend_from_slice(
            &self.edges_to_children[Self::RIGHT_HALF_SUB_NODES_STAR_INDEX as usize..],
        );
        let mut left_edges = self.edges_to_children;
        left_edges.truncate(Self::RIGHT_HALF_SUB_NODES_STAR_INDEX as usize);
        debug_assert_eq!(
            left_edges.len(),
            Self::NUMBER_OF_EDGES_DURING_SPLIT as usize
        );
        debug_assert_eq!(
            right_edges.len(),
            Self::NUMBER_OF_EDGES_DURING_SPLIT as usize
        );

        let left = Self::new_child(left_sub_nodes, left_edges)
            .expect("The number of keys in allowed range");
        let right = Self::new_child(right_sub_nodes, right_edges)
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

    fn edge(child_key: u8, child_hash: u8) -> Edge {
        Edge {
            key: [child_key; 32],
            hash: [child_hash; 32],
        }
    }

    #[test]
    fn root_node_can_be_created_with_1_leaf() {
        let _ = InternalNode::<2>::new_root(edge(0, 0), leaf(1, 1), edge(2, 2));
    }

    #[test]
    fn child_node_can_be_created_with_2_leafs_and_3_edges() {
        // When
        let result = InternalNode::<2>::new_child(
            vec![leaf(1, 1), leaf(3, 3)],
            vec![edge(0, 0), edge(2, 2), edge(4, 4)],
        );

        // Then
        let _node = result.expect("Should be able to create a node with 2 leafs");
    }

    #[test]
    fn child_node_creation_should_fails_with_2_leafs_and_2_edges() {
        // When
        let result = InternalNode::<2>::new_child(
            vec![leaf(1, 1), leaf(3, 3)],
            vec![edge(0, 0), edge(2, 2)],
        );

        // Then
        assert!(result.is_none());
    }

    #[test]
    fn child_node_creation_should_fails_with_3_leafs() {
        // When
        let result = InternalNode::<2>::new_child(
            vec![leaf(1, 1), leaf(3, 3), leaf(5, 5)],
            vec![edge(0, 0), edge(2, 2), edge(4, 4), edge(6, 6)],
        );

        // Then
        assert!(result.is_none());
    }

    #[test]
    fn insert_new_edge_to_root_node_adds_it() {
        // Given
        let node = InternalNode::<2>::new_root(edge(0, 0), leaf(1, 1), edge(2, 2));

        // When
        let result = node.insert_edge(edge(2, 2), leaf(3, 3), edge(4, 4));

        // Then
        let expected = InternalNode::<2>::new_child(
            vec![leaf(1, 1), leaf(3, 3)],
            vec![edge(0, 0), edge(2, 2), edge(4, 4)],
        )
        .unwrap();
        pretty_assertions::assert_eq!(
            result,
            InternalInsertionResult::Added {
                updated_node: expected,
            }
        );
    }

    #[test]
    fn insert_new_edge_to_root_node_adds_it_new_edge() {
        // Given
        let node = InternalNode::<2>::new_root(edge(0, 0), leaf(1, 1), edge(2, 2));

        // When
        let result = node.insert_edge(edge(3, 3), leaf(4, 4), edge(5, 5));

        // Then
        let expected = InternalNode::<2>::new_child(
            vec![leaf(1, 1), leaf(4, 4)],
            vec![edge(0, 0), edge(3, 3), edge(5, 5)],
        )
        .unwrap();
        pretty_assertions::assert_eq!(
            result,
            InternalInsertionResult::Added {
                updated_node: expected,
            }
        );
    }

    #[test]
    fn update_new_value_to_root_node_updates_it() {
        // Given
        let node = InternalNode::<2>::new_root(edge(0, 0), leaf(1, 1), edge(2, 2));

        // When
        let result = node.insert_edge(edge(0, 1), leaf(1, 2), edge(2, 3));

        // Then
        let expected = InternalNode::<2>::new_root(edge(0, 1), leaf(1, 2), edge(2, 3));
        pretty_assertions::assert_eq!(result, InternalInsertionResult::Updated(expected));
    }

    #[test]
    fn insert_new_value_to_full_child_node_2_splits_it() {
        // Given
        let node = InternalNode::<2>::new_child(
            vec![leaf(1, 1), leaf(5, 5)],
            vec![edge(0, 0), edge(4, 4), edge(6, 6)],
        )
        .unwrap();

        // When
        let result = node.insert_edge(edge(2, 2), leaf(3, 3), edge(4, 4));

        // Then
        let expected_left =
            InternalNode::<2>::new_child(vec![leaf(1, 1)], vec![edge(0, 0), edge(2, 2)])
                .unwrap();
        let expected_mid = leaf(3, 3);
        let expected_right =
            InternalNode::<2>::new_child(vec![leaf(5, 5)], vec![edge(4, 4), edge(6, 6)])
                .unwrap();
        pretty_assertions::assert_eq!(
            result,
            InternalInsertionResult::Overflowed {
                new_left_node: expected_left,
                orphan_sub_node: expected_mid,
                new_right_node: expected_right,
            }
        );
    }

    #[test]
    fn insert_new_value_to_full_child_node_4_splits_it() {
        // Given
        let node = InternalNode::<4>::new_child(
            vec![leaf(1, 1), leaf(3, 3), leaf(5, 5), leaf(7, 7)],
            vec![edge(0, 0), edge(2, 2), edge(4, 4), edge(6, 6), edge(8, 8)],
        )
        .unwrap();

        // When
        let result = node.insert_edge(edge(8, 8), leaf(9, 9), edge(10, 10));

        // Then
        let expected_left = InternalNode::<4>::new_child(
            vec![leaf(1, 1), leaf(3, 3)],
            vec![edge(0, 0), edge(2, 2), edge(4, 4)],
        )
        .unwrap();
        let expected_mid = leaf(5, 5);
        let expected_right = InternalNode::<4>::new_child(
            vec![leaf(7, 7), leaf(9, 9)],
            vec![edge(6, 6), edge(8, 8), edge(10, 10)],
        )
        .unwrap();
        pretty_assertions::assert_eq!(
            result,
            InternalInsertionResult::Overflowed {
                new_left_node: expected_left,
                orphan_sub_node: expected_mid,
                new_right_node: expected_right,
            }
        );
    }

    #[test]
    fn insert_2_values_to_root_node_splits_it_at_the_end() {
        // Given
        let node = InternalNode::<2>::new_root(edge(4, 4), leaf(5, 5), edge(6, 6));

        // When
        let InternalInsertionResult::Added { updated_node, .. } =
            node.insert_edge(edge(2, 2), leaf(3, 3), edge(4, 4))
        else {
            panic!("Should be able to add the second key");
        };
        let result = updated_node.insert_edge(edge(0, 0), leaf(1, 1), edge(2, 2));

        // Then
        let expected_left =
            InternalNode::<2>::new_child(vec![leaf(1, 1)], vec![edge(0, 0), edge(2, 2)])
                .unwrap();
        let expected_mid = leaf(3, 3);
        let expected_right =
            InternalNode::<2>::new_child(vec![leaf(5, 5)], vec![edge(4, 4), edge(6, 6)])
                .unwrap();
        pretty_assertions::assert_eq!(
            result,
            InternalInsertionResult::Overflowed {
                new_left_node: expected_left,
                orphan_sub_node: expected_mid,
                new_right_node: expected_right,
            }
        );
    }
}

use crate::{
    binary::{
        self,
        empty_sum,
        Node,
        Primitive,
    },
    common::{
        Bytes32,
        ProofSet,
        StorageMap,
    },
    storage::Mappable,
};

use crate::alloc::borrow::ToOwned;
use alloc::vec::Vec;

/// The table of the Binary Merkle Tree's nodes. [`MerkleTree`] works with it as
/// a binary array, where the storage key of the node is the `u64` index and
/// value is the [`Node`](crate::binary::Node).
#[derive(Debug, Clone)]
pub struct NodesTable;

impl Mappable for NodesTable {
    type Key = Self::OwnedKey;
    type OwnedKey = u64;
    type OwnedValue = Primitive;
    type Value = Self::OwnedValue;
}

type Storage = StorageMap<NodesTable>;
type BinaryMerkleTree = binary::MerkleTree<NodesTable, Storage>;

#[derive(Debug, Clone)]
pub struct MerkleTree {
    tree: BinaryMerkleTree,
}

impl MerkleTree {
    pub fn new() -> Self {
        Self {
            tree: BinaryMerkleTree::new(Storage::new()),
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        let _ = self.tree.push(data);
    }

    pub fn root(&self) -> Bytes32 {
        self.tree.root()
    }

    pub fn prove(&self, proof_index: u64) -> Option<(Bytes32, ProofSet)> {
        self.tree.prove(proof_index).ok()
    }

    pub fn reset(&mut self) {
        self.tree.reset();
    }
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MerkleRootCalculator {
    stack: Vec<Node>,
}

impl MerkleRootCalculator {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn push(&mut self, data: &[u8]) {
        let node = Node::create_leaf(0, data);
        self.stack.push(node);

        while let Some(top_node) = self.stack.last() {
            if self.stack.len() > 1 {
                if let Some(second_top_node) = self.stack.get(self.stack.len() - 2) {
                    if top_node.height() == second_top_node.height() {
                        let merged_node = Node::create_node(second_top_node, top_node);
                        self.stack.pop();
                        self.stack.pop();
                        self.stack.push(merged_node);
                    } else {
                        break
                    }
                }
            } else {
                break
            }
        }
    }

    pub fn root(&mut self) -> Bytes32 {
        if self.stack.is_empty() {
            return empty_sum().to_owned()
        }

        while self.stack.len() > 1 {
            let right_child = self.stack.pop().expect("Unable to pop element from stack");
            let left_child = self.stack.pop().expect("Unable to pop element from stack");
            let merged_node = Node::create_node(&left_child, &right_child);
            self.stack.push(merged_node);
        }

        self.stack.pop().unwrap().hash().to_owned()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use binary::{
        empty_sum,
        leaf_sum,
        node_sum,
    };
    use fuel_merkle_test_helpers::TEST_DATA;

    #[test]
    fn root_returns_the_empty_root_for_0_leaves() {
        let tree = MerkleTree::new();
        let mut calculate_root = MerkleRootCalculator::new();

        let root = tree.root();
        assert_eq!(root, empty_sum().clone());
        let root = calculate_root.root();
        assert_eq!(root, empty_sum().clone());
    }

    #[test]
    fn root_returns_the_merkle_root_for_1_leaf() {
        let mut tree = MerkleTree::new();
        let mut calculate_root = MerkleRootCalculator::new();

        let data = &TEST_DATA[0..1]; // 1 leaf
        for datum in data.iter() {
            tree.push(datum);
            calculate_root.push(datum)
        }

        let leaf_0 = leaf_sum(data[0]);

        let root = tree.root();
        assert_eq!(root, leaf_0);
        assert_eq!(root, calculate_root.root());
    }

    #[test]
    fn root_returns_the_merkle_root_for_7_leaves() {
        let mut tree = MerkleTree::new();
        let mut calculate_root = MerkleRootCalculator::new();

        let data = &TEST_DATA[0..7]; // 7 leaves
        for datum in data.iter() {
            tree.push(datum);
            calculate_root.push(datum)
        }

        //               07
        //              /  \
        //             /    \
        //            /      \
        //           /        \
        //          /          \
        //         /            \
        //       03              11
        //      /  \            /  \
        //     /    \          /    \
        //   01      05      09      \
        //  /  \    /  \    /  \      \
        // 00  02  04  06  08  10     12
        // 00  01  02  03  04  05     06

        let leaf_0 = leaf_sum(data[0]);
        let leaf_1 = leaf_sum(data[1]);
        let leaf_2 = leaf_sum(data[2]);
        let leaf_3 = leaf_sum(data[3]);
        let leaf_4 = leaf_sum(data[4]);
        let leaf_5 = leaf_sum(data[5]);
        let leaf_6 = leaf_sum(data[6]);

        let node_1 = node_sum(&leaf_0, &leaf_1);
        let node_5 = node_sum(&leaf_2, &leaf_3);
        let node_3 = node_sum(&node_1, &node_5);
        let node_9 = node_sum(&leaf_4, &leaf_5);
        let node_11 = node_sum(&node_9, &leaf_6);
        let node_7 = node_sum(&node_3, &node_11);

        let root = tree.root();
        assert_eq!(root, node_7);
        assert_eq!(root, calculate_root.root());
    }

    #[test]
    fn prove_returns_none_for_0_leaves() {
        let tree = MerkleTree::new();

        let proof = tree.prove(0);
        assert!(proof.is_none());
    }

    #[test]
    fn prove_returns_none_when_index_is_greater_than_number_of_leaves() {
        let mut tree = MerkleTree::new();

        let data = &TEST_DATA[0..5]; // 5 leaves
        for datum in data.iter() {
            tree.push(datum);
        }

        let proof = tree.prove(10);
        assert!(proof.is_none());
    }

    #[test]
    fn prove_returns_the_merkle_root_and_proof_set_for_1_leaf() {
        let mut tree = MerkleTree::new();

        let data = &TEST_DATA[0..1]; // 1 leaf
        for datum in data.iter() {
            tree.push(datum);
        }

        let leaf_0 = leaf_sum(data[0]);

        {
            let (root, proof_set) = tree.prove(0).unwrap();
            assert_eq!(root, leaf_0);
            assert!(proof_set.is_empty());
        }
    }

    #[test]
    fn prove_returns_the_merkle_root_and_proof_set_for_7_leaves() {
        let mut tree = MerkleTree::new();

        let data = &TEST_DATA[0..7]; // 7 leaves
        for datum in data.iter() {
            tree.push(datum);
        }

        //               07
        //              /  \
        //             /    \
        //            /      \
        //           /        \
        //          /          \
        //         /            \
        //       03              11
        //      /  \            /  \
        //     /    \          /    \
        //   01      05      09      \
        //  /  \    /  \    /  \      \
        // 00  02  04  06  08  10     12
        // 00  01  02  03  04  05     06

        let leaf_0 = leaf_sum(data[0]);
        let leaf_1 = leaf_sum(data[1]);
        let leaf_2 = leaf_sum(data[2]);
        let leaf_3 = leaf_sum(data[3]);
        let leaf_4 = leaf_sum(data[4]);
        let leaf_5 = leaf_sum(data[5]);
        let leaf_6 = leaf_sum(data[6]);

        let node_1 = node_sum(&leaf_0, &leaf_1);
        let node_5 = node_sum(&leaf_2, &leaf_3);
        let node_3 = node_sum(&node_1, &node_5);
        let node_9 = node_sum(&leaf_4, &leaf_5);
        let node_11 = node_sum(&node_9, &leaf_6);
        let node_7 = node_sum(&node_3, &node_11);

        {
            let (root, proof_set) = tree.prove(0).unwrap();
            assert_eq!(root, node_7);
            assert_eq!(proof_set[0], leaf_1);
            assert_eq!(proof_set[1], node_5);
            assert_eq!(proof_set[2], node_11);
        }
        {
            let (root, proof_set) = tree.prove(1).unwrap();
            assert_eq!(root, node_7);
            assert_eq!(proof_set[0], leaf_0);
            assert_eq!(proof_set[1], node_5);
            assert_eq!(proof_set[2], node_11);
        }
        {
            let (root, proof_set) = tree.prove(2).unwrap();
            assert_eq!(root, node_7);
            assert_eq!(proof_set[0], leaf_3);
            assert_eq!(proof_set[1], node_1);
            assert_eq!(proof_set[2], node_11);
        }
        {
            let (root, proof_set) = tree.prove(3).unwrap();
            assert_eq!(root, node_7);
            assert_eq!(proof_set[0], leaf_2);
            assert_eq!(proof_set[1], node_1);
            assert_eq!(proof_set[2], node_11);
        }
        {
            let (root, proof_set) = tree.prove(4).unwrap();
            assert_eq!(root, node_7);
            assert_eq!(proof_set[0], leaf_5);
            assert_eq!(proof_set[1], leaf_6);
            assert_eq!(proof_set[2], node_3);
        }
        {
            let (root, proof_set) = tree.prove(5).unwrap();
            assert_eq!(root, node_7);
            assert_eq!(proof_set[0], leaf_4);
            assert_eq!(proof_set[1], leaf_6);
            assert_eq!(proof_set[2], node_3);
        }
        {
            let (root, proof_set) = tree.prove(6).unwrap();
            assert_eq!(root, node_7);
            assert_eq!(proof_set[0], node_9);
            assert_eq!(proof_set[1], node_3);
        }
    }
}

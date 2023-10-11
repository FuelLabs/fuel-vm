use crate::{
    binary::{
        empty_sum,
        Node,
    },
    common::Bytes32,
};

use crate::alloc::borrow::ToOwned;
use alloc::vec::Vec;

#[derive(Default, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MerkleRootCalculator {
    stack: Vec<Node>,
}

impl MerkleRootCalculator {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn new_with_stack(stack: Vec<Node>) -> Self {
        Self { stack }
    }

    pub fn push(&mut self, data: &[u8]) {
        let node = Node::create_leaf(0, data);
        self.stack.push(node);

        while self.stack.len() > 1 {
            let right_node = &self.stack[self.stack.len() - 1];
            let left_node = &self.stack[self.stack.len() - 2];
            if right_node.height() == left_node.height() {
                let merged_node = Node::create_node(left_node, right_node);
                self.stack.pop();
                self.stack.pop();
                self.stack.push(merged_node);
            } else {
                break
            }
        }
    }

    pub fn root(mut self) -> Bytes32 {
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

    pub fn root_from_iterator<I: Iterator<Item = T>, T: AsRef<[u8]>>(
        self,
        iterator: I,
    ) -> Bytes32 {
        let mut calculator = MerkleRootCalculator::new();

        for data in iterator {
            calculator.push(data.as_ref());
        }

        calculator.root()
    }

    pub fn stack(&self) -> &Vec<Node> {
        &self.stack
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::binary::in_memory::MerkleTree;
    use fuel_merkle_test_helpers::TEST_DATA;
    #[cfg(test)]
    use serde_json as _;

    #[test]
    fn root_returns_the_empty_root_for_0_leaves() {
        let tree = MerkleTree::new();
        let calculate_root = MerkleRootCalculator::new();

        assert_eq!(tree.root(), calculate_root.root());
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

        assert_eq!(tree.root(), calculate_root.root());
    }

    #[test]
    fn root_returns_the_merkle_root_for_7_leaves() {
        let mut tree = MerkleTree::new();
        let mut calculate_root = MerkleRootCalculator::new();

        let data = &TEST_DATA[0..7];
        for datum in data.iter() {
            tree.push(datum);
            calculate_root.push(datum)
        }
        assert_eq!(tree.root(), calculate_root.root());
    }

    #[test]
    fn root_returns_the_merkle_root_for_100000_leaves() {
        let mut tree = MerkleTree::new();
        let mut calculate_root = MerkleRootCalculator::new();

        for value in 0..10000u64 {
            let data = value.to_le_bytes();
            tree.push(&data);
            calculate_root.push(&data);
        }

        assert_eq!(tree.root(), calculate_root.root());
    }

    #[test]
    fn root_returns_the_merkle_root_from_iterator() {
        let mut tree = MerkleTree::new();
        let calculate_root = MerkleRootCalculator::new();

        let data = &TEST_DATA[0..7];
        for datum in data.iter() {
            tree.push(datum);
        }

        let root = calculate_root.root_from_iterator(data.iter());

        assert_eq!(tree.root(), root);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_serialize_deserialize() {
        let mut calculator = MerkleRootCalculator::new();

        let data = &TEST_DATA[0..7];
        for datum in data.iter() {
            calculator.push(datum);
        }
        let json = serde_json::to_string(&calculator).unwrap();

        let deserialized_calculator: MerkleRootCalculator =
            serde_json::from_str(&json).expect("Unable to read from str");

        assert_eq!(calculator.root(), deserialized_calculator.root());
    }
}

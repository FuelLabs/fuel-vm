use crate::binary::{leaf_sum, node_sum};
use crate::common::{Bytes32, Position};

use core::fmt::Debug;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Node {
    position: Position,
    hash: Bytes32,
}

impl Node {
    pub fn create_leaf(index: u64, data: &[u8]) -> Self {
        let position = Position::from_leaf_index(index);
        let hash = leaf_sum(data);
        Self { position, hash }
    }

    pub fn create_node(left_child: &Self, right_child: &Self) -> Self {
        let position = left_child.position().parent();
        let hash = node_sum(left_child.hash(), right_child.hash());
        Self { position, hash }
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn key(&self) -> u64 {
        self.position().in_order_index()
    }

    pub fn hash(&self) -> &Bytes32 {
        &self.hash
    }
}

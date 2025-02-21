use crate::{
    binary::{
        leaf_sum,
        node_sum,
    },
    common::{
        Bytes32,
        Position,
    },
};

use core::fmt::Debug;
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Node {
    position: Position,
    hash: Bytes32,
}

impl Node {
    pub fn new(position: Position, hash: Bytes32) -> Self {
        Self { position, hash }
    }

    /// Returns `None` if the leaf cannot be created due to incorrect position.
    pub fn create_leaf(index: u64, data: &[u8]) -> Option<Self> {
        let position = Position::from_leaf_index(index)?;
        let hash = leaf_sum(data);
        Some(Self { position, hash })
    }

    /// Returns `None` if the leaf cannot be created due to incorrect position.
    pub fn create_leaf_with_hash(index: u64, hash: Bytes32) -> Option<Self> {
        let position = Position::from_leaf_index(index)?;
        Some(Self { position, hash })
    }

    /// Creates a new node with the given children.
    pub fn create_node(
        position: Position,
        left_child: &Self,
        right_child: &Self,
    ) -> Self {
        let hash = node_sum(left_child.hash(), right_child.hash());
        Self { position, hash }
    }

    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn key(&self) -> u64 {
        self.position().in_order_index()
    }

    pub fn hash(&self) -> &Bytes32 {
        &self.hash
    }

    pub fn height(&self) -> u32 {
        self.position().height()
    }
}

impl AsRef<Node> for Node {
    fn as_ref(&self) -> &Node {
        self
    }
}

use crate::{
    common::Bytes32,
    sum::{
        leaf_sum,
        node_sum,
    },
};
use core::fmt;

#[derive(Clone)]
pub struct Node {
    height: u32,
    hash: Bytes32,
    fee: u64,
    left_child_key: Option<Bytes32>,
    right_child_key: Option<Bytes32>,
}

impl Node {
    pub fn create_leaf(fee: u64, data: &[u8]) -> Self {
        Self {
            height: 0,
            hash: leaf_sum(fee, data),
            fee,
            left_child_key: None,
            right_child_key: None,
        }
    }

    pub fn create_node(
        height: u32,
        lhs_fee: u64,
        lhs_key: &Bytes32,
        rhs_fee: u64,
        rhs_key: &Bytes32,
    ) -> Self {
        Self {
            height,
            hash: node_sum(lhs_fee, lhs_key, rhs_fee, rhs_key),
            fee: lhs_fee
                .checked_add(rhs_fee)
                .expect("Program should panic if this overflows"),
            left_child_key: Some(*lhs_key),
            right_child_key: Some(*rhs_key),
        }
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn hash(&self) -> &Bytes32 {
        &self.hash
    }

    pub fn fee(&self) -> u64 {
        self.fee
    }

    pub fn left_child_key(&self) -> Option<Bytes32> {
        self.left_child_key
    }

    pub fn right_child_key(&self) -> Option<Bytes32> {
        self.right_child_key
    }

    pub fn is_leaf(&self) -> bool {
        self.height == 0
    }

    pub fn is_node(&self) -> bool {
        !self.is_leaf()
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_node() {
            f.debug_struct("Node (Internal)")
                .field("Hash", &hex::encode(self.hash()))
                .field("Fee", &self.fee)
                .field(
                    "Left child key",
                    &hex::encode(self.left_child_key().unwrap()),
                )
                .field(
                    "Right child key",
                    &hex::encode(self.right_child_key().unwrap()),
                )
                .finish()
        } else {
            f.debug_struct("Node (Leaf)")
                .field("Hash", &hex::encode(self.hash()))
                .field("Fee", &self.fee)
                .field("Key", &hex::encode(self.hash()))
                .finish()
        }
    }
}

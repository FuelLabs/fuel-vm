use crate::binary::{leaf_sum, node_sum};
use fuel_storage::Storage;
use std::fmt::Debug;

use crate::common::{Bytes32, Position};

#[derive(Clone, PartialEq, Debug)]
pub struct Node {
    position: Position,
    key: Bytes32,
    parent_key: Option<Bytes32>,
    left_key: Option<Bytes32>,
    right_key: Option<Bytes32>,
}

impl Node {
    pub fn create_leaf(index: u64, data: &[u8]) -> Self {
        let position = Position::from_leaf_index(index);
        let key = leaf_sum(data);
        Self {
            position,
            key,
            parent_key: None,
            left_key: None,
            right_key: None,
        }
    }

    pub fn create_node(left_child: &mut Self, right_child: &mut Self) -> Self {
        let position = left_child.position().parent();
        let key = node_sum(&left_child.key(), &right_child.key());
        let node = Self {
            position,
            key,
            parent_key: None,
            left_key: Some(left_child.key()),
            right_key: Some(right_child.key()),
        };
        left_child.set_parent_key(Some(node.key()));
        right_child.set_parent_key(Some(node.key()));
        node
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn key(&self) -> Bytes32 {
        self.key
    }

    pub fn parent_key(&self) -> Option<Bytes32> {
        self.parent_key.clone()
    }

    pub fn left_key(&self) -> Option<Bytes32> {
        self.left_key.clone()
    }

    pub fn right_key(&self) -> Option<Bytes32> {
        self.right_key.clone()
    }

    pub fn proof_iter<'storage, StorageError: std::error::Error>(
        &mut self,
        storage: &'storage dyn Storage<Bytes32, Self, Error = StorageError>,
    ) -> ProofIter<'storage, StorageError> {
        ProofIter::new(storage, self)
    }

    fn set_parent_key(&mut self, key: Option<Bytes32>) {
        self.parent_key = key;
    }
}

pub struct ProofIter<'storage, StorageError> {
    storage: &'storage dyn Storage<Bytes32, Node, Error = StorageError>,
    prev: Option<Node>,
    curr: Option<Node>,
}

impl<'storage, StorageError> ProofIter<'storage, StorageError>
where
    StorageError: std::error::Error,
{
    pub fn new(
        storage: &'storage dyn Storage<Bytes32, Node, Error = StorageError>,
        node: &Node,
    ) -> Self {
        let parent_key = node.parent_key();
        match parent_key {
            None => Self {
                storage,
                prev: Some(node.clone()),
                curr: None,
            },
            Some(key) => {
                let curr = storage.get(&key).unwrap().unwrap();
                Self {
                    storage,
                    prev: Some(node.clone()),
                    curr: Some(curr.into_owned()),
                }
            }
        }
    }
}

impl<'storage, StorageError> Iterator for ProofIter<'storage, StorageError>
where
    StorageError: std::error::Error,
{
    type Item = Node;

    fn next(&mut self) -> Option<Self::Item> {
        let previous = self.prev.take();
        let mut current = self.curr.take();

        let node = current.as_ref().map(|curr| {
            let prev = previous.unwrap();
            if curr.left_key().unwrap() == prev.key() {
                self.storage
                    .get(&curr.right_key().unwrap())
                    .unwrap()
                    .unwrap()
                    .into_owned()
            } else {
                self.storage
                    .get(&curr.left_key().unwrap())
                    .unwrap()
                    .unwrap()
                    .into_owned()
            }
        });

        self.curr = current
            .as_ref()?
            .parent_key()
            .map(|key| self.storage.get(&key).unwrap().unwrap().into_owned());
        self.prev = current.take();

        node
    }
}

#[cfg(test)]
mod test {
    use crate::binary::Node;
    use crate::common::{Bytes32, StorageMap};
    use fuel_merkle_test_helpers::TEST_DATA;
    use fuel_storage::Storage;

    #[test]
    pub fn test_proof_iter() {
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
        //   01      05       09     \
        //  /  \    /  \     /  \     \
        // 00  02  04  06   08  10    12
        // 00  01  02  03   04  05    06

        let mut leaf_0 = Node::create_leaf(0, TEST_DATA[0]);
        let mut leaf_1 = Node::create_leaf(1, TEST_DATA[1]);
        let mut leaf_2 = Node::create_leaf(2, TEST_DATA[2]);
        let mut leaf_3 = Node::create_leaf(3, TEST_DATA[3]);
        let mut leaf_4 = Node::create_leaf(4, TEST_DATA[4]);
        let mut leaf_5 = Node::create_leaf(5, TEST_DATA[5]);
        let mut leaf_6 = Node::create_leaf(6, TEST_DATA[6]);

        let mut node_1 = Node::create_node(&mut leaf_0, &mut leaf_1);
        let mut node_5 = Node::create_node(&mut leaf_2, &mut leaf_3);
        let mut node_9 = Node::create_node(&mut leaf_4, &mut leaf_5);
        let mut node_3 = Node::create_node(&mut node_1, &mut node_5);
        let mut node_11 = Node::create_node(&mut node_9, &mut leaf_6);
        let node_7 = Node::create_node(&mut node_3, &mut node_11);

        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let _ = storage_map.insert(&leaf_1.key(), &leaf_1);
        let _ = storage_map.insert(&leaf_2.key(), &leaf_2);
        let _ = storage_map.insert(&leaf_0.key(), &leaf_0);
        let _ = storage_map.insert(&leaf_3.key(), &leaf_3);
        let _ = storage_map.insert(&leaf_4.key(), &leaf_4);
        let _ = storage_map.insert(&leaf_5.key(), &leaf_5);
        let _ = storage_map.insert(&leaf_6.key(), &leaf_6);
        let _ = storage_map.insert(&node_1.key(), &node_1);
        let _ = storage_map.insert(&node_5.key(), &node_5);
        let _ = storage_map.insert(&node_9.key(), &node_9);
        let _ = storage_map.insert(&node_3.key(), &node_3);
        let _ = storage_map.insert(&node_11.key(), &node_11);
        let _ = storage_map.insert(&node_7.key(), &node_7);

        let iter = leaf_0.proof_iter(&mut storage_map);
        let col: Vec<Node> = iter.collect();
        assert_eq!(col, vec!(leaf_1.clone(), node_5.clone(), node_11.clone()));

        let iter = leaf_1.proof_iter(&mut storage_map);
        let col: Vec<Node> = iter.collect();
        assert_eq!(col, vec!(leaf_0.clone(), node_5.clone(), node_11.clone()));

        let iter = leaf_2.proof_iter(&mut storage_map);
        let col: Vec<Node> = iter.collect();
        assert_eq!(col, vec!(leaf_3.clone(), node_1.clone(), node_11.clone()));

        let iter = leaf_3.proof_iter(&mut storage_map);
        let col: Vec<Node> = iter.collect();
        assert_eq!(col, vec!(leaf_2.clone(), node_1.clone(), node_11.clone()));

        let iter = leaf_4.proof_iter(&mut storage_map);
        let col: Vec<Node> = iter.collect();
        assert_eq!(col, vec!(leaf_5.clone(), leaf_6.clone(), node_3.clone()));

        let iter = leaf_5.proof_iter(&mut storage_map);
        let col: Vec<Node> = iter.collect();
        assert_eq!(col, vec!(leaf_4.clone(), leaf_6.clone(), node_3.clone()));

        let iter = leaf_6.proof_iter(&mut storage_map);
        let col: Vec<Node> = iter.collect();
        assert_eq!(col, vec!(node_9.clone(), node_3.clone()));
    }
}

use fuel_storage::Storage;
use std::fmt::Debug;

use crate::common::Position;

#[derive(Clone, PartialEq, Debug)]
pub struct Node<Key> {
    position: Position,
    key: Key,
    parent_key: Option<Key>,
    left_key: Option<Key>,
    right_key: Option<Key>,
}

impl<Key> Node<Key>
where
    Key: Clone,
{
    pub fn new(position: Position, key: Key) -> Self {
        Self {
            position,
            key,
            parent_key: None,
            left_key: None,
            right_key: None,
        }
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn key(&self) -> Key {
        self.key.clone()
    }

    pub fn parent_key(&self) -> Option<Key> {
        self.parent_key.clone()
    }

    pub fn left_key(&self) -> Option<Key> {
        self.left_key.clone()
    }

    pub fn right_key(&self) -> Option<Key> {
        self.right_key.clone()
    }

    pub fn set_parent_key(&mut self, key: Option<Key>) {
        self.parent_key = key;
    }

    pub fn set_left_key(&mut self, key: Option<Key>) {
        self.left_key = key;
    }

    pub fn set_right_key(&mut self, key: Option<Key>) {
        self.right_key = key;
    }

    pub fn proof_iter<'storage, StorageError: std::error::Error>(
        &mut self,
        storage: &'storage dyn Storage<Key, Self, Error = StorageError>,
    ) -> ProofIter<'storage, Key, StorageError> {
        ProofIter::new(storage, self)
    }
}

pub struct ProofIter<'storage, Key, StorageError> {
    storage: &'storage dyn Storage<Key, Node<Key>, Error = StorageError>,
    prev: Option<Node<Key>>,
    curr: Option<Node<Key>>,
}

impl<'storage, Key, StorageError> ProofIter<'storage, Key, StorageError>
where
    Key: Clone,
    StorageError: std::error::Error,
{
    pub fn new(
        storage: &'storage dyn Storage<Key, Node<Key>, Error = StorageError>,
        node: &Node<Key>,
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

impl<'storage, Key, StorageError> Iterator for ProofIter<'storage, Key, StorageError>
where
    Key: Clone + std::cmp::PartialEq,
    StorageError: std::error::Error,
{
    type Item = Node<Key>;

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
    use crate::common::{Position, StorageMap};
    use fuel_storage::Storage;

    #[test]
    pub fn test_proof_iter() {
        type N = Node<u32>;

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

        let mut leaf_0 = N::new(Position::from_leaf_index(0), 0);
        let mut leaf_1 = N::new(Position::from_leaf_index(1), 2);
        let mut leaf_2 = N::new(Position::from_leaf_index(2), 4);
        let mut leaf_3 = N::new(Position::from_leaf_index(3), 6);
        let mut leaf_4 = N::new(Position::from_leaf_index(4), 8);
        let mut leaf_5 = N::new(Position::from_leaf_index(5), 10);
        let mut leaf_6 = N::new(Position::from_leaf_index(6), 12);

        let mut node_1 = N::new(Position::from_in_order_index(1), 1);
        leaf_0.set_parent_key(Some(node_1.key()));
        leaf_1.set_parent_key(Some(node_1.key()));
        node_1.set_left_key(Some(leaf_0.key()));
        node_1.set_right_key(Some(leaf_1.key()));

        let mut node_5 = N::new(Position::from_in_order_index(5), 5);
        leaf_2.set_parent_key(Some(node_5.key()));
        leaf_3.set_parent_key(Some(node_5.key()));
        node_5.set_left_key(Some(leaf_2.key()));
        node_5.set_right_key(Some(leaf_3.key()));

        let mut node_9 = N::new(Position::from_in_order_index(9), 9);
        leaf_4.set_parent_key(Some(node_9.key()));
        leaf_5.set_parent_key(Some(node_9.key()));
        node_9.set_left_key(Some(leaf_4.key()));
        node_9.set_right_key(Some(leaf_5.key()));

        let mut node_3 = N::new(Position::from_in_order_index(3), 3);
        node_1.set_parent_key(Some(node_3.key()));
        node_5.set_parent_key(Some(node_3.key()));
        node_3.set_left_key(Some(node_1.key()));
        node_3.set_right_key(Some(node_5.key()));

        let mut node_11 = N::new(Position::from_in_order_index(11), 11);
        node_9.set_parent_key(Some(node_11.key()));
        leaf_6.set_parent_key(Some(node_11.key()));
        node_11.set_left_key(Some(node_9.key()));
        node_11.set_right_key(Some(leaf_6.key()));

        let mut node_7 = N::new(Position::from_in_order_index(7), 7);
        node_3.set_parent_key(Some(node_7.key()));
        node_11.set_parent_key(Some(node_7.key()));
        node_7.set_left_key(Some(node_3.key()));
        node_7.set_right_key(Some(node_11.key()));

        let mut storage_map = StorageMap::<u32, N>::new();
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
        let col: Vec<N> = iter.collect();
        assert_eq!(col, vec!(leaf_1.clone(), node_5.clone(), node_11.clone()));

        let iter = leaf_1.proof_iter(&mut storage_map);
        let col: Vec<N> = iter.collect();
        assert_eq!(col, vec!(leaf_0.clone(), node_5.clone(), node_11.clone()));

        let iter = leaf_2.proof_iter(&mut storage_map);
        let col: Vec<N> = iter.collect();
        assert_eq!(col, vec!(leaf_3.clone(), node_1.clone(), node_11.clone()));

        let iter = leaf_3.proof_iter(&mut storage_map);
        let col: Vec<N> = iter.collect();
        assert_eq!(col, vec!(leaf_2.clone(), node_1.clone(), node_11.clone()));

        let iter = leaf_4.proof_iter(&mut storage_map);
        let col: Vec<N> = iter.collect();
        assert_eq!(col, vec!(leaf_5.clone(), leaf_6.clone(), node_3.clone()));

        let iter = leaf_5.proof_iter(&mut storage_map);
        let col: Vec<N> = iter.collect();
        assert_eq!(col, vec!(leaf_4.clone(), leaf_6.clone(), node_3.clone()));

        let iter = leaf_6.proof_iter(&mut storage_map);
        let col: Vec<N> = iter.collect();
        assert_eq!(col, vec!(node_9.clone(), node_3.clone()));
    }
}

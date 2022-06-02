use crate::binary::{self, Node};
use crate::common::{Bytes32, Position, Subtree};

use fuel_storage::Storage;

use alloc::boxed::Box;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum MerkleTreeError<StorageError> {
    #[cfg_attr(feature = "std", error("proof index {0} is not valid"))]
    InvalidProofIndex(u64),

    #[cfg_attr(
        feature = "std",
        error("cannot load node with key {0}; the key is not found in storage")
    )]
    LoadError(u64),

    #[cfg_attr(feature = "std", error("a storage error was thrown: {0}"))]
    StorageError(StorageError),
}

impl<StorageError> From<StorageError> for MerkleTreeError<StorageError> {
    fn from(err: StorageError) -> MerkleTreeError<StorageError> {
        MerkleTreeError::StorageError(err)
    }
}

type ProofSet = Vec<Bytes32>;

pub struct MerkleTree<'storage, StorageType> {
    storage: &'storage mut StorageType,
    head: Option<Box<Subtree<Node>>>,
    leaves_count: u64,
}

impl<'storage, StorageType, StorageError> MerkleTree<'storage, StorageType>
where
    StorageType: Storage<u64, Node, Error = StorageError>,
    StorageError: 'static,
{
    pub fn new(storage: &'storage mut StorageType) -> Self {
        Self {
            storage,
            head: None,
            leaves_count: 0,
        }
    }

    pub fn load(
        storage: &'storage mut StorageType,
        leaves_count: u64,
    ) -> Result<Self, MerkleTreeError<StorageError>> {
        let mut tree = Self {
            storage,
            head: None,
            leaves_count,
        };

        tree.build()?;

        Ok(tree)
    }

    pub fn root(&mut self) -> Result<Bytes32, MerkleTreeError<StorageError>> {
        let root_node = self.root_node()?;
        let root = match root_node {
            None => *binary::empty_sum(),
            Some(ref node) => *node.hash(),
        };

        Ok(root)
    }

    pub fn prove(
        &mut self,
        proof_index: u64,
    ) -> Result<(Bytes32, ProofSet), MerkleTreeError<StorageError>> {
        if proof_index + 1 > self.leaves_count {
            return Err(MerkleTreeError::InvalidProofIndex(proof_index).into());
        }

        let mut proof_set = ProofSet::new();

        let root_node = self.root_node()?.unwrap();
        let root_position = root_node.position();
        let leaf_position = Position::from_leaf_index(proof_index);
        let leaf_node = self.storage.get(&leaf_position.in_order_index())?.unwrap();
        proof_set.push(*leaf_node.hash());

        let (_, mut side_positions): (Vec<_>, Vec<_>) = root_position
            .path(&leaf_position, self.leaves_count)
            .iter()
            .unzip();
        side_positions.reverse(); // Reorder side positions from leaf to root.
        side_positions.pop(); // The last side position is the root; remove it.

        for side_position in side_positions {
            let key = side_position.in_order_index();
            let node = self.storage.get(&key)?.unwrap();
            proof_set.push(*node.hash());
        }

        let root = *root_node.hash();
        Ok((root, proof_set))
    }

    pub fn push(&mut self, data: &[u8]) -> Result<(), MerkleTreeError<StorageError>> {
        let node = Node::create_leaf(self.leaves_count, data);
        self.storage.insert(&node.key(), &node)?;
        let next = self.head.take();
        let head = Box::new(Subtree::<Node>::new(node, next));
        self.head = Some(head);
        self.join_all_subtrees()?;

        self.leaves_count += 1;

        Ok(())
    }

    //
    // PRIVATE
    //

    fn build(&mut self) -> Result<(), MerkleTreeError<StorageError>> {
        let keys = (0..self.leaves_count).map(|i| Position::from_leaf_index(i).in_order_index());
        for key in keys {
            let node = self
                .storage
                .get(&key)?
                .ok_or(MerkleTreeError::LoadError(key))?
                .into_owned();
            let next = self.head.take();
            let head = Box::new(Subtree::<Node>::new(node, next));
            self.head = Some(head);
            self.join_all_subtrees()?;
        }

        Ok(())
    }

    fn root_node(&mut self) -> Result<Option<Node>, StorageError> {
        let root_node = match self.head {
            None => None,
            Some(ref initial) => {
                let mut current = initial.clone();
                while current.next().is_some() {
                    let mut head = current;
                    let mut head_next = head.take_next().unwrap();
                    current = self.join_subtrees(&mut head_next, &mut head)?
                }
                Some(current.node().clone())
            }
        };

        Ok(root_node)
    }

    fn join_all_subtrees(&mut self) -> Result<(), StorageError> {
        loop {
            let current = self.head.as_ref().unwrap();
            if !(current.next().is_some()
                && current.node().position().height()
                    == current.next_node().unwrap().position().height())
            {
                break;
            }

            // Merge the two front heads of the list into a single head
            let joined_head = {
                let mut head = self.head.take().unwrap();
                let mut head_next = head.take_next().unwrap();
                self.join_subtrees(&mut head_next, &mut head)?
            };
            self.head = Some(joined_head);
        }

        Ok(())
    }

    fn join_subtrees(
        &mut self,
        lhs: &mut Subtree<Node>,
        rhs: &mut Subtree<Node>,
    ) -> Result<Box<Subtree<Node>>, StorageError> {
        let joined_node = Node::create_node(lhs.node(), rhs.node());
        self.storage.insert(&joined_node.key(), &joined_node)?;
        let joined_head = Subtree::new(joined_node, lhs.take_next());
        Ok(Box::new(joined_head))
    }
}

#[cfg(test)]
mod test {
    use super::{MerkleTree, Storage};
    use crate::binary::{empty_sum, leaf_sum, node_sum, Node};
    use crate::common::StorageMap;
    use fuel_merkle_test_helpers::TEST_DATA;

    #[test]
    fn test_push_builds_internal_tree_structure() {
        let mut storage_map = StorageMap::<u64, Node>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let data = &TEST_DATA[0..7]; // 7 leaves
        for datum in data.iter() {
            let _ = tree.push(datum);
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

        let leaf_0 = leaf_sum(&data[0]);
        let leaf_1 = leaf_sum(&data[1]);
        let leaf_2 = leaf_sum(&data[2]);
        let leaf_3 = leaf_sum(&data[3]);
        let leaf_4 = leaf_sum(&data[4]);
        let leaf_5 = leaf_sum(&data[5]);
        let leaf_6 = leaf_sum(&data[6]);
        let node_1 = node_sum(&leaf_0, &leaf_1);
        let node_5 = node_sum(&leaf_2, &leaf_3);
        let node_3 = node_sum(&node_1, &node_5);
        let node_9 = node_sum(&leaf_4, &leaf_5);

        let s_leaf_0 = storage_map.get(&0).unwrap().unwrap();
        let s_leaf_1 = storage_map.get(&2).unwrap().unwrap();
        let s_leaf_2 = storage_map.get(&4).unwrap().unwrap();
        let s_leaf_3 = storage_map.get(&6).unwrap().unwrap();
        let s_leaf_4 = storage_map.get(&8).unwrap().unwrap();
        let s_leaf_5 = storage_map.get(&10).unwrap().unwrap();
        let s_leaf_6 = storage_map.get(&12).unwrap().unwrap();
        let s_node_1 = storage_map.get(&1).unwrap().unwrap();
        let s_node_5 = storage_map.get(&5).unwrap().unwrap();
        let s_node_9 = storage_map.get(&9).unwrap().unwrap();
        let s_node_3 = storage_map.get(&3).unwrap().unwrap();

        assert_eq!(s_leaf_0.hash(), &leaf_0);
        assert_eq!(s_leaf_1.hash(), &leaf_1);
        assert_eq!(s_leaf_2.hash(), &leaf_2);
        assert_eq!(s_leaf_3.hash(), &leaf_3);
        assert_eq!(s_leaf_4.hash(), &leaf_4);
        assert_eq!(s_leaf_5.hash(), &leaf_5);
        assert_eq!(s_leaf_6.hash(), &leaf_6);
        assert_eq!(s_node_1.hash(), &node_1);
        assert_eq!(s_node_5.hash(), &node_5);
        assert_eq!(s_node_9.hash(), &node_9);
        assert_eq!(s_node_3.hash(), &node_3);
    }

    #[test]
    fn load_returns_a_valid_tree() {
        const LEAVES_COUNT: u64 = 7;

        let mut storage_map = StorageMap::<u64, Node>::new();

        let root_1 = {
            let mut tree = MerkleTree::new(&mut storage_map);
            let data = &TEST_DATA[0..LEAVES_COUNT as usize];
            for datum in data.iter() {
                let _ = tree.push(datum);
            }
            tree.root().unwrap()
        };

        let root_2 = {
            let mut tree = MerkleTree::load(&mut storage_map, LEAVES_COUNT).unwrap();
            tree.root().unwrap()
        };

        assert_eq!(root_1, root_2);
    }

    #[test]
    fn load_returns_a_load_error_if_the_storage_is_not_valid_for_the_leaves_count() {
        let mut storage_map = StorageMap::<u64, Node>::new();

        {
            let mut tree = MerkleTree::new(&mut storage_map);
            let data = &TEST_DATA[0..5];
            for datum in data.iter() {
                let _ = tree.push(datum);
            }
        }

        {
            let tree = MerkleTree::load(&mut storage_map, 10);
            assert!(tree.is_err());
        }
    }

    #[test]
    fn root_returns_the_empty_root_for_0_leaves() {
        let mut storage_map = StorageMap::<u64, Node>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let root = tree.root().unwrap();
        assert_eq!(root, empty_sum().clone());
    }

    #[test]
    fn root_returns_the_merkle_root_for_1_leaf() {
        let mut storage_map = StorageMap::<u64, Node>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let data = &TEST_DATA[0..1]; // 1 leaf
        for datum in data.iter() {
            let _ = tree.push(datum);
        }

        let leaf_0 = leaf_sum(&data[0]);

        let root = tree.root().unwrap();
        assert_eq!(root, leaf_0);
    }

    #[test]
    fn root_returns_the_merkle_root_for_7_leaves() {
        let mut storage_map = StorageMap::<u64, Node>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let data = &TEST_DATA[0..7]; // 7 leaves
        for datum in data.iter() {
            let _ = tree.push(datum);
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

        let leaf_0 = leaf_sum(&data[0]);
        let leaf_1 = leaf_sum(&data[1]);
        let leaf_2 = leaf_sum(&data[2]);
        let leaf_3 = leaf_sum(&data[3]);
        let leaf_4 = leaf_sum(&data[4]);
        let leaf_5 = leaf_sum(&data[5]);
        let leaf_6 = leaf_sum(&data[6]);

        let node_1 = node_sum(&leaf_0, &leaf_1);
        let node_5 = node_sum(&leaf_2, &leaf_3);
        let node_3 = node_sum(&node_1, &node_5);
        let node_9 = node_sum(&leaf_4, &leaf_5);
        let node_11 = node_sum(&node_9, &leaf_6);
        let node_7 = node_sum(&node_3, &node_11);

        let root = tree.root().unwrap();
        assert_eq!(root, node_7);
    }

    #[test]
    fn prove_returns_invalid_proof_index_error_for_0_leaves() {
        let mut storage_map = StorageMap::<u64, Node>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let proof = tree.prove(0);
        assert!(proof.is_err());
    }

    #[test]
    fn prove_returns_invalid_proof_index_error_when_index_is_greater_than_number_of_leaves() {
        let mut storage_map = StorageMap::<u64, Node>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let data = &TEST_DATA[0..5]; // 5 leaves
        for datum in data.iter() {
            let _ = tree.push(datum);
        }

        let proof = tree.prove(10);
        assert!(proof.is_err());
    }

    #[test]
    fn prove_returns_the_merkle_root_and_proof_set_for_1_leaf() {
        let mut storage_map = StorageMap::<u64, Node>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let data = &TEST_DATA[0..1]; // 1 leaf
        for datum in data.iter() {
            let _ = tree.push(datum);
        }

        let leaf_0 = leaf_sum(&data[0]);

        {
            let proof = tree.prove(0).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, leaf_0);
            assert_eq!(set[0], leaf_0);
        }
    }

    #[test]
    fn prove_returns_the_merkle_root_and_proof_set_for_4_leaves() {
        let mut storage_map = StorageMap::<u64, Node>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let data = &TEST_DATA[0..4]; // 4 leaves
        for datum in data.iter() {
            let _ = tree.push(datum);
        }

        //       03
        //      /  \
        //     /    \
        //   01      05
        //  /  \    /  \
        // 00  02  04  06
        // 00  01  02  03

        let leaf_0 = leaf_sum(&data[0]);
        let leaf_1 = leaf_sum(&data[1]);
        let leaf_2 = leaf_sum(&data[2]);
        let leaf_3 = leaf_sum(&data[3]);

        let node_1 = node_sum(&leaf_0, &leaf_1);
        let node_5 = node_sum(&leaf_2, &leaf_3);
        let node_3 = node_sum(&node_1, &node_5);

        {
            let proof = tree.prove(0).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_3);
            assert_eq!(set[0], leaf_0);
            assert_eq!(set[1], leaf_1);
            assert_eq!(set[2], node_5);
        }
        {
            let proof = tree.prove(1).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_3);
            assert_eq!(set[0], leaf_1);
            assert_eq!(set[1], leaf_0);
            assert_eq!(set[2], node_5);
        }
        {
            let proof = tree.prove(2).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_3);
            assert_eq!(set[0], leaf_2);
            assert_eq!(set[1], leaf_3);
            assert_eq!(set[2], node_1);
        }
        {
            let proof = tree.prove(3).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_3);
            assert_eq!(set[0], leaf_3);
            assert_eq!(set[1], leaf_2);
            assert_eq!(set[2], node_1);
        }
    }

    #[test]
    fn prove_returns_the_merkle_root_and_proof_set_for_5_leaves() {
        let mut storage_map = StorageMap::<u64, Node>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let data = &TEST_DATA[0..5]; // 5 leaves
        for datum in data.iter() {
            let _ = tree.push(datum);
        }

        //          07
        //          /\
        //         /  \
        //       03    \
        //      /  \    \
        //     /    \    \
        //   01      05   \
        //  /  \    /  \   \
        // 00  02  04  06  08
        // 00  01  02  03  04

        let leaf_0 = leaf_sum(&data[0]);
        let leaf_1 = leaf_sum(&data[1]);
        let leaf_2 = leaf_sum(&data[2]);
        let leaf_3 = leaf_sum(&data[3]);
        let leaf_4 = leaf_sum(&data[4]);

        let node_1 = node_sum(&leaf_0, &leaf_1);
        let node_5 = node_sum(&leaf_2, &leaf_3);
        let node_3 = node_sum(&node_1, &node_5);
        let node_7 = node_sum(&node_3, &leaf_4);

        {
            let proof = tree.prove(0).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_7);
            assert_eq!(set[0], leaf_0);
            assert_eq!(set[1], leaf_1);
            assert_eq!(set[2], node_5);
            assert_eq!(set[3], leaf_4);
        }
        {
            let proof = tree.prove(1).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_7);
            assert_eq!(set[0], leaf_1);
            assert_eq!(set[1], leaf_0);
            assert_eq!(set[2], node_5);
            assert_eq!(set[3], leaf_4);
        }
        {
            let proof = tree.prove(2).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_7);
            assert_eq!(set[0], leaf_2);
            assert_eq!(set[1], leaf_3);
            assert_eq!(set[2], node_1);
            assert_eq!(set[3], leaf_4);
        }
        {
            let proof = tree.prove(3).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_7);
            assert_eq!(set[0], leaf_3);
            assert_eq!(set[1], leaf_2);
            assert_eq!(set[2], node_1);
            assert_eq!(set[3], leaf_4);
        }
        {
            let proof = tree.prove(4).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_7);
            assert_eq!(set[0], leaf_4);
            assert_eq!(set[1], node_3);
        }
    }

    #[test]
    fn prove_returns_the_merkle_root_and_proof_set_for_7_leaves() {
        let mut storage_map = StorageMap::<u64, Node>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let data = &TEST_DATA[0..7]; // 7 leaves
        for datum in data.iter() {
            let _ = tree.push(datum);
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

        let leaf_0 = leaf_sum(&data[0]);
        let leaf_1 = leaf_sum(&data[1]);
        let leaf_2 = leaf_sum(&data[2]);
        let leaf_3 = leaf_sum(&data[3]);
        let leaf_4 = leaf_sum(&data[4]);
        let leaf_5 = leaf_sum(&data[5]);
        let leaf_6 = leaf_sum(&data[6]);

        let node_1 = node_sum(&leaf_0, &leaf_1);
        let node_5 = node_sum(&leaf_2, &leaf_3);
        let node_3 = node_sum(&node_1, &node_5);
        let node_9 = node_sum(&leaf_4, &leaf_5);
        let node_11 = node_sum(&node_9, &leaf_6);
        let node_7 = node_sum(&node_3, &node_11);

        {
            let proof = tree.prove(0).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_7);
            assert_eq!(set[0], leaf_0);
            assert_eq!(set[1], leaf_1);
            assert_eq!(set[2], node_5);
            assert_eq!(set[3], node_11);
        }
        {
            let proof = tree.prove(1).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_7);
            assert_eq!(set[0], leaf_1);
            assert_eq!(set[1], leaf_0);
            assert_eq!(set[2], node_5);
            assert_eq!(set[3], node_11);
        }
        {
            let proof = tree.prove(2).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_7);
            assert_eq!(set[0], leaf_2);
            assert_eq!(set[1], leaf_3);
            assert_eq!(set[2], node_1);
            assert_eq!(set[3], node_11);
        }
        {
            let proof = tree.prove(3).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_7);
            assert_eq!(set[0], leaf_3);
            assert_eq!(set[1], leaf_2);
            assert_eq!(set[2], node_1);
            assert_eq!(set[3], node_11);
        }
        {
            let proof = tree.prove(4).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_7);
            assert_eq!(set[0], leaf_4);
            assert_eq!(set[1], leaf_5);
            assert_eq!(set[2], leaf_6);
            assert_eq!(set[3], node_3);
        }
        {
            let proof = tree.prove(5).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_7);
            assert_eq!(set[0], leaf_5);
            assert_eq!(set[1], leaf_4);
            assert_eq!(set[2], leaf_6);
            assert_eq!(set[3], node_3);
        }
        {
            let proof = tree.prove(6).unwrap();
            let root = proof.0;
            let set = proof.1;

            assert_eq!(root, node_7);
            assert_eq!(set[0], leaf_6);
            assert_eq!(set[1], node_9);
            assert_eq!(set[2], node_3);
        }
    }
}

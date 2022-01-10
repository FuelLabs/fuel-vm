use fuel_storage::Storage;

use crate::binary::{empty_sum, Node};
use crate::common::{Bytes32, Subtree};

#[derive(Debug, thiserror::Error)]
pub enum MerkleTreeError {
    #[error("proof index {0} is not valid")]
    InvalidProofIndex(u64),
}

type ProofSet = Vec<Bytes32>;

pub struct MerkleTree<'storage, StorageError> {
    storage: &'storage mut dyn Storage<Bytes32, Node, Error = StorageError>,
    head: Option<Box<Subtree<Node>>>,
    leaves: Vec<Bytes32>,
    leaves_count: u64,
}

impl<'storage, StorageError> MerkleTree<'storage, StorageError>
where
    StorageError: std::error::Error + 'static,
{
    pub fn new(storage: &'storage mut dyn Storage<Bytes32, Node, Error = StorageError>) -> Self {
        Self {
            storage,
            head: None,
            leaves: Vec::<Bytes32>::default(),
            leaves_count: 0,
        }
    }

    pub fn root(&mut self) -> Result<Bytes32, Box<dyn std::error::Error>> {
        let root = match self.head {
            None => *empty_sum(),
            Some(ref initial) => {
                let mut current = initial.clone();
                while current.next().is_some() {
                    let mut head = current;
                    let mut head_next = head.take_next().unwrap();
                    current = self.join_subtrees(&mut head_next, &mut head)?
                }
                current.node().key()
            }
        };

        Ok(root)
    }

    pub fn prove(
        &mut self,
        proof_index: u64,
    ) -> Result<(Bytes32, ProofSet), Box<dyn std::error::Error>> {
        if proof_index + 1 > self.leaves_count {
            return Err(Box::new(MerkleTreeError::InvalidProofIndex(proof_index)));
        }

        let root = self.root()?;
        let mut proof_set = ProofSet::new();

        let key = self.leaves[proof_index as usize];
        proof_set.push(key);

        let mut node = self.storage.get(&key)?.unwrap();
        let iter = node.to_mut().proof_iter(self.storage);
        for n in iter {
            proof_set.push(n.key());
        }

        Ok((root, proof_set))
    }

    pub fn push(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let node = Node::create_leaf(self.leaves_count, data);
        self.storage.insert(&node.key(), &node)?;
        self.leaves.push(node.key());

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

    fn join_all_subtrees(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    ) -> Result<Box<Subtree<Node>>, Box<dyn std::error::Error>> {
        let joined_node = Node::create_node(lhs.node_mut(), rhs.node_mut());
        self.storage.insert(&joined_node.key(), &joined_node)?;
        self.storage.insert(&lhs.node().key(), lhs.node())?;
        self.storage.insert(&rhs.node().key(), rhs.node())?;

        let joined_head = Subtree::new(joined_node, lhs.take_next());
        Ok(Box::new(joined_head))
    }
}

#[cfg(test)]
mod test {
    use super::{MerkleTree, Storage};
    use crate::binary::{empty_sum, leaf_sum, node_sum, Node};
    use crate::common::{Bytes32, StorageError, StorageMap};
    use fuel_merkle_test_helpers::TEST_DATA;

    type MT<'a> = MerkleTree<'a, StorageError>;

    #[test]
    fn test_push_builds_internal_tree_structure() {
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

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
        //   01      05       09     \
        //  /  \    /  \     /  \     \
        // 00  02  04  06   08  10    12
        // 00  01  02  03   04  05    06

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

        let s_leaf_0 = storage_map.get(&leaf_0).unwrap().unwrap();
        assert_eq!(s_leaf_0.left_key(), None);
        assert_eq!(s_leaf_0.right_key(), None);
        assert_eq!(s_leaf_0.parent_key(), Some(node_1.clone()));

        let s_leaf_1 = storage_map.get(&leaf_1).unwrap().unwrap();
        assert_eq!(s_leaf_1.left_key(), None);
        assert_eq!(s_leaf_1.right_key(), None);
        assert_eq!(s_leaf_1.parent_key(), Some(node_1.clone()));

        let s_leaf_2 = storage_map.get(&leaf_2).unwrap().unwrap();
        assert_eq!(s_leaf_2.left_key(), None);
        assert_eq!(s_leaf_2.right_key(), None);
        assert_eq!(s_leaf_2.parent_key(), Some(node_5.clone()));

        let s_leaf_3 = storage_map.get(&leaf_3).unwrap().unwrap();
        assert_eq!(s_leaf_3.left_key(), None);
        assert_eq!(s_leaf_3.right_key(), None);
        assert_eq!(s_leaf_3.parent_key(), Some(node_5.clone()));

        let s_leaf_4 = storage_map.get(&leaf_4).unwrap().unwrap();
        assert_eq!(s_leaf_4.left_key(), None);
        assert_eq!(s_leaf_4.right_key(), None);
        assert_eq!(s_leaf_4.parent_key(), Some(node_9.clone()));

        let s_leaf_5 = storage_map.get(&leaf_5).unwrap().unwrap();
        assert_eq!(s_leaf_5.left_key(), None);
        assert_eq!(s_leaf_5.right_key(), None);
        assert_eq!(s_leaf_5.parent_key(), Some(node_9.clone()));

        let s_leaf_6 = storage_map.get(&leaf_6).unwrap().unwrap();
        assert_eq!(s_leaf_6.left_key(), None);
        assert_eq!(s_leaf_6.right_key(), None);
        assert_eq!(s_leaf_6.parent_key(), None);

        let s_node_1 = storage_map.get(&node_1).unwrap().unwrap();
        assert_eq!(s_node_1.left_key(), Some(leaf_0.clone()));
        assert_eq!(s_node_1.right_key(), Some(leaf_1.clone()));
        assert_eq!(s_node_1.parent_key(), Some(node_3.clone()));

        let s_node_5 = storage_map.get(&node_5).unwrap().unwrap();
        assert_eq!(s_node_5.left_key(), Some(leaf_2.clone()));
        assert_eq!(s_node_5.right_key(), Some(leaf_3.clone()));
        assert_eq!(s_node_5.parent_key(), Some(node_3.clone()));

        let s_node_9 = storage_map.get(&node_9).unwrap().unwrap();
        assert_eq!(s_node_9.left_key(), Some(leaf_4.clone()));
        assert_eq!(s_node_9.right_key(), Some(leaf_5.clone()));
        assert_eq!(s_node_9.parent_key(), None);

        let s_node_3 = storage_map.get(&node_3).unwrap().unwrap();
        assert_eq!(s_node_3.left_key(), Some(node_1.clone()));
        assert_eq!(s_node_3.right_key(), Some(node_5.clone()));
        assert_eq!(s_node_3.parent_key(), None);
    }

    #[test]
    fn root_returns_the_empty_root_for_0_leaves() {
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

        let root = tree.root().unwrap();
        assert_eq!(root, empty_sum().clone());
    }

    #[test]
    fn root_returns_the_merkle_root_for_1_leaf() {
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

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
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

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
        //   01      05       09     \
        //  /  \    /  \     /  \     \
        // 00  02  04  06   08  10    12
        // 00  01  02  03   04  05    06

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
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

        let proof = tree.prove(0);
        assert!(proof.is_err());
    }

    #[test]
    fn prove_returns_invalid_proof_index_error_when_index_is_greater_than_number_of_leaves() {
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

        let data = &TEST_DATA[0..5]; // 5 leaves
        for datum in data.iter() {
            let _ = tree.push(datum);
        }

        let proof = tree.prove(10);
        assert!(proof.is_err());
    }

    #[test]
    fn prove_returns_the_merkle_root_and_proof_set_for_1_leaf() {
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

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
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

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
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

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
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

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
        //   01      05       09     \
        //  /  \    /  \     /  \     \
        // 00  02  04  06   08  10    12
        // 00  01  02  03   04  05    06

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

use crate::binary::{
    empty_sum,
    leaf_sum,
    node_sum,
    Data,
    Node,
};
use std::collections::VecDeque;

type DataNode = Node<Data>;
type ProofSet = Vec<Data>;

#[derive(Default)]
pub struct MerkleTree {
    head: Option<Box<DataNode>>,
    leaves_count: u64,
    proof_index: u64,
    proof_set: ProofSet,
}

impl MerkleTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_proof_index(&mut self, proof_index: u64) {
        if self.head().is_some() {
            panic!("Cannot change the proof index after adding a leaf!");
        }
        self.proof_index = proof_index;
    }

    pub fn root(&self) -> Data {
        match self.head() {
            None => *empty_sum(),
            Some(head) => {
                let mut current = head.clone();
                while current.next().is_some() {
                    let mut node = current;
                    let mut next_node = node.take_next().unwrap();
                    current = Self::join_subtrees(&mut next_node, &node)
                }
                *current.data()
            }
        }
    }

    pub fn leaves_count(&self) -> u64 {
        self.leaves_count
    }

    pub fn push(&mut self, data: &[u8]) {
        if self.leaves_count == self.proof_index {
            self.proof_set.push(leaf_sum(data));
        }

        let node = Self::create_node(self.head.take(), 0, leaf_sum(data));
        self.head = Some(node);
        self.join_all_subtrees();

        self.leaves_count += 1;
    }

    pub fn prove(mut self) -> (Data, ProofSet) {
        let proof_set_length = self.proof_set.len() as u32;

        if self.head().is_none() || proof_set_length == 0 {
            return (self.root(), self.proof_set)
        }

        let mut current = self.head().clone().unwrap();
        while current.next().is_some()
            && current.next_height().unwrap() < proof_set_length - 1
        {
            let mut node = current;
            let mut next_node = node.take_next().unwrap();
            current = Self::join_subtrees(&mut next_node, &node)
        }

        if current.next().is_some()
            && current.next_height().unwrap() == proof_set_length - 1
        {
            self.proof_set.push(*current.data());
            current = current.take_next().unwrap();
        }

        while current.next().is_some() {
            self.proof_set.push(*current.next_data().unwrap());
            current = current.take_next().unwrap();
        }

        let root = self.root();

        // The proof set starts with the hashed leaf data; remove the leaf data
        // in order to conform with the proof set specification.
        let mut proof_set = VecDeque::from(self.proof_set);
        proof_set.pop_front();

        (root, proof_set.into())
    }

    fn head(&self) -> &Option<Box<DataNode>> {
        &self.head
    }

    fn join_all_subtrees(&mut self) {
        loop {
            let head = self.head.as_ref().unwrap();
            if !(head.next().is_some() && head.height() == head.next_height().unwrap()) {
                break
            }

            let proof_set_length = self.proof_set.len() as u32;
            if head.height() + 1 == proof_set_length {
                let head_leaves_count = 1u64 << head.height();
                let mid = (self.leaves_count / head_leaves_count) * head_leaves_count;
                if self.proof_index < mid {
                    self.proof_set.push(*head.data());
                } else {
                    self.proof_set.push(*head.next_data().unwrap());
                }
            }

            // Merge the two front nodes of the list into a single node
            let mut node = self.head.take().unwrap();
            let mut next_node = node.take_next().unwrap();
            let joined_node = Self::join_subtrees(&mut next_node, &node);

            self.head = Some(joined_node);
        }
    }

    fn join_subtrees(a: &mut DataNode, b: &DataNode) -> Box<DataNode> {
        let next = a.take_next();
        let height = a.height() + 1;
        let data = node_sum(a.data(), b.data());
        Self::create_node(next, height, data)
    }

    fn create_node(
        next: Option<Box<DataNode>>,
        height: u32,
        data: Data,
    ) -> Box<DataNode> {
        Box::new(DataNode::new(next, height, data))
    }
}

#[cfg(test)]
mod test {
    use super::MerkleTree;
    use crate::{
        binary::{
            empty_sum,
            leaf_sum,
            node_sum,
        },
        TEST_DATA,
    };

    #[test]
    fn root_returns_the_hash_of_the_empty_string_when_no_leaves_are_pushed() {
        let mt = MerkleTree::new();
        let root = mt.root();

        let expected = empty_sum();
        assert_eq!(&root, expected);
    }

    #[test]
    fn root_returns_the_hash_of_the_leaf_when_one_leaf_is_pushed() {
        let mut mt = MerkleTree::new();

        let data = &TEST_DATA[0];
        mt.push(data);
        let root = mt.root();

        let expected = leaf_sum(data);
        assert_eq!(root, expected);
    }

    #[test]
    fn root_returns_the_hash_of_the_head_when_4_leaves_are_pushed() {
        let mut mt = MerkleTree::new();

        let data = &TEST_DATA[0..4]; // 4 leaves
        for datum in data.iter() {
            mt.push(datum);
        }
        let root = mt.root();

        //       N3
        //      /  \
        //     /    \
        //   N1      N2
        //  /  \    /  \
        // L1  L2  L3  L4

        let leaf_1 = leaf_sum(data[0]);
        let leaf_2 = leaf_sum(data[1]);
        let leaf_3 = leaf_sum(data[2]);
        let leaf_4 = leaf_sum(data[3]);

        let node_1 = node_sum(&leaf_1, &leaf_2);
        let node_2 = node_sum(&leaf_3, &leaf_4);
        let node_3 = node_sum(&node_1, &node_2);

        let expected = node_3;
        assert_eq!(root, expected);
    }

    #[test]
    fn root_returns_the_hash_of_the_head_when_5_leaves_are_pushed() {
        let mut mt = MerkleTree::new();

        let data = &TEST_DATA[0..5]; // 5 leaves
        for datum in data.iter() {
            mt.push(datum);
        }
        let root = mt.root();

        //          N4
        //         /  \
        //       N3    \
        //      /  \    \
        //     /    \    \
        //   N1      N2   \
        //  /  \    /  \   \
        // L1  L2  L3  L4  L5

        let leaf_1 = leaf_sum(data[0]);
        let leaf_2 = leaf_sum(data[1]);
        let leaf_3 = leaf_sum(data[2]);
        let leaf_4 = leaf_sum(data[3]);
        let leaf_5 = leaf_sum(data[4]);

        let node_1 = node_sum(&leaf_1, &leaf_2);
        let node_2 = node_sum(&leaf_3, &leaf_4);
        let node_3 = node_sum(&node_1, &node_2);
        let node_4 = node_sum(&node_3, &leaf_5);

        let expected = node_4;
        assert_eq!(root, expected);
    }

    #[test]
    fn root_returns_the_hash_of_the_head_when_7_leaves_are_pushed() {
        let mut mt = MerkleTree::new();

        let data = &TEST_DATA[0..7]; // 7 leaves
        for datum in data.iter() {
            mt.push(datum);
        }
        let root = mt.root();

        //              N6
        //          /        \
        //         /          \
        //       N4            N5
        //      /  \           /\
        //     /    \         /  \
        //   N1      N2      N3   \
        //  /  \    /  \    /  \   \
        // L1  L2  L3  L4  L5  L6  L7

        let leaf_1 = leaf_sum(data[0]);
        let leaf_2 = leaf_sum(data[1]);
        let leaf_3 = leaf_sum(data[2]);
        let leaf_4 = leaf_sum(data[3]);
        let leaf_5 = leaf_sum(data[4]);
        let leaf_6 = leaf_sum(data[5]);
        let leaf_7 = leaf_sum(data[6]);

        let node_1 = node_sum(&leaf_1, &leaf_2);
        let node_2 = node_sum(&leaf_3, &leaf_4);
        let node_3 = node_sum(&leaf_5, &leaf_6);
        let node_4 = node_sum(&node_1, &node_2);
        let node_5 = node_sum(&node_3, &leaf_7);
        let node_6 = node_sum(&node_4, &node_5);

        let expected = node_6;
        assert_eq!(root, expected);
    }

    #[test]
    fn leaves_count_returns_the_number_of_leaves_pushed_to_the_tree() {
        let mut mt = MerkleTree::new();

        let data = &TEST_DATA[0..4];
        for datum in data.iter() {
            mt.push(datum);
        }

        assert_eq!(mt.leaves_count(), data.len() as u64);
    }

    #[test]
    fn prove_returns_the_merkle_root_and_proof_set_for_the_given_proof_index() {
        let mut mt = MerkleTree::new();
        mt.set_proof_index(0);

        let data = &TEST_DATA[0..4]; // 4 leaves
        for datum in data.iter() {
            mt.push(datum);
        }

        let (root, proof_set) = mt.prove();

        //       N3
        //      /  \
        //     /    \
        //   N1      N2
        //  /  \    /  \
        // L1  L2  L3  L4

        let leaf_1 = leaf_sum(data[0]);
        let leaf_2 = leaf_sum(data[1]);
        let leaf_3 = leaf_sum(data[2]);
        let leaf_4 = leaf_sum(data[3]);

        let node_1 = node_sum(&leaf_1, &leaf_2);
        let node_2 = node_sum(&leaf_3, &leaf_4);
        let node_3 = node_sum(&node_1, &node_2);

        assert_eq!(root, node_3);
        assert_eq!(proof_set[0], leaf_2);
        assert_eq!(proof_set[1], node_2);
    }

    #[test]
    fn prove_returns_the_merkle_root_and_proof_set_for_the_given_proof_index_left_of_the_root(
    ) {
        let mut mt = MerkleTree::new();
        mt.set_proof_index(2);

        let data = &TEST_DATA[0..5]; // 5 leaves
        for datum in data.iter() {
            mt.push(datum);
        }

        let (root, proof_set) = mt.prove();

        //          N4
        //         /  \
        //       N3    \
        //      /  \    \
        //     /    \    \
        //   N1      N2   \
        //  /  \    /  \   \
        // L1  L2  L3  L4  L5

        let leaf_1 = leaf_sum(data[0]);
        let leaf_2 = leaf_sum(data[1]);
        let leaf_3 = leaf_sum(data[2]);
        let leaf_4 = leaf_sum(data[3]);
        let leaf_5 = leaf_sum(data[4]);

        let node_1 = node_sum(&leaf_1, &leaf_2);
        let node_2 = node_sum(&leaf_3, &leaf_4);
        let node_3 = node_sum(&node_1, &node_2);
        let node_4 = node_sum(&node_3, &leaf_5);

        assert_eq!(root, node_4);
        assert_eq!(proof_set[0], leaf_4);
        assert_eq!(proof_set[1], node_1);
        assert_eq!(proof_set[2], leaf_5);
    }

    #[test]
    fn prove_returns_the_merkle_root_and_proof_set_for_the_given_proof_index_right_of_the_root(
    ) {
        let mut mt = MerkleTree::new();
        mt.set_proof_index(4);

        let data = &TEST_DATA[0..5]; // 5 leaves
        for datum in data.iter() {
            mt.push(datum);
        }

        let (root, proof_set) = mt.prove();

        //          N4
        //         /  \
        //       N3    \
        //      /  \    \
        //     /    \    \
        //   N1      N2   \
        //  /  \    /  \   \
        // L1  L2  L3  L4  L5

        let leaf_1 = leaf_sum(data[0]);
        let leaf_2 = leaf_sum(data[1]);
        let leaf_3 = leaf_sum(data[2]);
        let leaf_4 = leaf_sum(data[3]);
        let leaf_5 = leaf_sum(data[4]);

        let node_1 = node_sum(&leaf_1, &leaf_2);
        let node_2 = node_sum(&leaf_3, &leaf_4);
        let node_3 = node_sum(&node_1, &node_2);
        let node_4 = node_sum(&node_3, &leaf_5);

        assert_eq!(root, node_4);
        assert_eq!(proof_set[0], node_3);
    }

    #[test]
    fn prove_returns_the_root_of_the_empty_merkle_tree_when_no_leaves_are_added() {
        let mt = MerkleTree::new();
        let (root, _proof_set) = mt.prove();
        let expected_root = empty_sum();
        assert_eq!(&root, expected_root);
    }

    #[test]
    fn prove_returns_an_empty_proof_set_when_no_leaves_are_added() {
        let mt = MerkleTree::new();
        let (_root, proof_set) = mt.prove();
        assert!(proof_set.is_empty());
    }
}

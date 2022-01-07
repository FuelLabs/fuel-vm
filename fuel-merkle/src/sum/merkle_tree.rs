use fuel_storage::Storage;

use crate::common::{Bytes32, Subtree};
use crate::sum::{empty_sum, Node};

#[derive(Debug, thiserror::Error)]
pub enum MerkleTreeError {
    #[error("proof index {0} is not valid")]
    InvalidProofIndex(u64),
}

pub struct MerkleTree<'storage, StorageError> {
    storage: &'storage mut dyn Storage<Bytes32, Node, Error = StorageError>,
    head: Option<Box<Subtree<Node>>>,
}

impl<'storage, StorageError> MerkleTree<'storage, StorageError>
where
    StorageError: 'static + std::error::Error + Clone,
{
    pub fn new(storage: &'storage mut dyn Storage<Bytes32, Node, Error = StorageError>) -> Self {
        Self {
            storage,
            head: None,
        }
    }

    pub fn root(&mut self) -> Result<(u64, Bytes32), Box<dyn std::error::Error>> {
        let root_node = self.root_node()?;
        let root_pair = match root_node {
            None => (0, *empty_sum()),
            Some(ref node) => (node.fee(), *node.hash()),
        };

        Ok(root_pair)
    }

    pub fn push(&mut self, fee: u64, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let node = Node::create_leaf(fee, data);
        self.storage.insert(node.hash(), &node)?;

        let next = self.head.take();
        let head = Box::new(Subtree::<Node>::new(node, next));
        self.head = Some(head);
        self.join_all_subtrees()?;

        Ok(())
    }

    //
    // PRIVATE
    //

    fn root_node(&mut self) -> Result<Option<Node>, Box<dyn std::error::Error>> {
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

    fn join_all_subtrees(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let current = self.head.as_ref().unwrap();
            if !(current.next().is_some()
                && current.node().height() == current.next_node().unwrap().height())
            {
                break;
            }

            // Merge the two front nodes of the list into a single node
            let joined_node = {
                let mut head = self.head.take().unwrap();
                let mut head_next = head.take_next().unwrap();
                self.join_subtrees(&mut head_next, &mut head)?
            };
            self.head = Some(joined_node);
        }

        Ok(())
    }

    fn join_subtrees(
        &mut self,
        lhs: &mut Subtree<Node>,
        rhs: &mut Subtree<Node>,
    ) -> Result<Box<Subtree<Node>>, Box<dyn std::error::Error>> {
        let height = lhs.node().height() + 1;
        let joined_node = Node::create_node(
            height,
            lhs.node().fee(),
            lhs.node().hash(),
            rhs.node().fee(),
            rhs.node().hash(),
        );
        self.storage.insert(joined_node.hash(), &joined_node)?;

        let joined_head = Subtree::new(joined_node, lhs.take_next());

        Ok(Box::new(joined_head))
    }
}

#[cfg(test)]
mod test {
    use fuel_merkle_test_helpers::TEST_DATA;

    use crate::common::{Bytes32, StorageError, StorageMap};
    use crate::sum::{empty_sum, leaf_sum, node_sum, MerkleTree, Node};

    type MT<'storage> = MerkleTree<'storage, StorageError>;
    const FEE: u64 = 100;

    #[test]
    fn root_returns_the_hash_of_the_empty_string_when_no_leaves_are_pushed() {
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

        let root = tree.root().unwrap();
        assert_eq!(root, (0, empty_sum().clone()));
    }

    #[test]
    fn root_returns_the_hash_of_the_leaf_when_one_leaf_is_pushed() {
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

        let data = &TEST_DATA[0];
        let _ = tree.push(FEE, &data);
        let root = tree.root().unwrap();

        let expected = (FEE, leaf_sum(FEE, &data));
        assert_eq!(root, expected);
    }

    #[test]
    fn root_returns_the_hash_of_the_head_when_4_leaves_are_pushed() {
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

        let data = &TEST_DATA[0..4]; // 4 leaves
        for datum in data.iter() {
            let _ = tree.push(FEE, datum);
        }
        let root = tree.root().unwrap();

        //       N2
        //      /  \
        //     /    \
        //   N0      N1
        //  /  \    /  \
        // L0  L1  L2  L3

        let leaf_0 = leaf_sum(FEE, &data[0]);
        let leaf_1 = leaf_sum(FEE, &data[1]);
        let leaf_2 = leaf_sum(FEE, &data[2]);
        let leaf_3 = leaf_sum(FEE, &data[3]);

        let node_0 = node_sum(FEE * 1, &leaf_0, FEE * 1, &leaf_1);
        let node_1 = node_sum(FEE * 1, &leaf_2, FEE * 1, &leaf_3);
        let node_2 = node_sum(FEE * 2, &node_0, FEE * 2, &node_1);

        let expected = (FEE * 4, node_2);
        assert_eq!(root, expected);
    }

    #[test]
    fn root_returns_the_hash_of_the_head_when_5_leaves_are_pushed() {
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

        let data = &TEST_DATA[0..5]; // 5 leaves
        for datum in data.iter() {
            let _ = tree.push(FEE, datum);
        }
        let root = tree.root().unwrap();

        //          N3
        //         /  \
        //       N2    \
        //      /  \    \
        //     /    \    \
        //   N0      N1   \
        //  /  \    /  \   \
        // L0  L1  L2  L3  L4

        let leaf_0 = leaf_sum(FEE, &data[0]);
        let leaf_1 = leaf_sum(FEE, &data[1]);
        let leaf_2 = leaf_sum(FEE, &data[2]);
        let leaf_3 = leaf_sum(FEE, &data[3]);
        let leaf_4 = leaf_sum(FEE, &data[4]);

        let node_0 = node_sum(FEE * 1, &leaf_0, FEE * 1, &leaf_1);
        let node_1 = node_sum(FEE * 1, &leaf_2, FEE * 1, &leaf_3);
        let node_2 = node_sum(FEE * 2, &node_0, FEE * 2, &node_1);
        let node_3 = node_sum(FEE * 4, &node_2, FEE * 1, &leaf_4);

        let expected = (FEE * 5, node_3);
        assert_eq!(root, expected);
    }

    #[test]
    fn root_returns_the_hash_of_the_head_when_7_leaves_are_pushed() {
        let mut storage_map = StorageMap::<Bytes32, Node>::new();
        let mut tree = MT::new(&mut storage_map);

        let data = &TEST_DATA[0..7]; // 7 leaves
        for datum in data.iter() {
            let _ = tree.push(FEE, datum);
        }
        let root = tree.root().unwrap();

        //              N5
        //            /    \
        //           /      \
        //          /        \
        //         /          \
        //       N3            N4
        //      /  \           /\
        //     /    \         /  \
        //   N0      N1      N2   \
        //  /  \    /  \    /  \   \
        // L0  L1  L2  L3  L4  L5  L6

        let leaf_0 = leaf_sum(FEE, &data[0]);
        let leaf_1 = leaf_sum(FEE, &data[1]);
        let leaf_2 = leaf_sum(FEE, &data[2]);
        let leaf_3 = leaf_sum(FEE, &data[3]);
        let leaf_4 = leaf_sum(FEE, &data[4]);
        let leaf_5 = leaf_sum(FEE, &data[5]);
        let leaf_6 = leaf_sum(FEE, &data[6]);

        let node_0 = node_sum(FEE * 1, &leaf_0, FEE * 1, &leaf_1);
        let node_1 = node_sum(FEE * 1, &leaf_2, FEE * 1, &leaf_3);
        let node_2 = node_sum(FEE * 1, &leaf_4, FEE * 1, &leaf_5);
        let node_3 = node_sum(FEE * 2, &node_0, FEE * 2, &node_1);
        let node_4 = node_sum(FEE * 2, &node_2, FEE * 1, &leaf_6);
        let node_5 = node_sum(FEE * 4, &node_3, FEE * 3, &node_4);

        let expected = (FEE * 7, node_5);
        assert_eq!(root, expected);
    }
}

use crate::{
    binary::{empty_sum, in_memory::NodesTable, Node, Primitive},
    common::{Bytes32, Position, ProofSet, StorageMap, Subtree},
    storage::{
        Mappable, StorageInspect, StorageInspectInfallible, StorageMutate, StorageMutateInfallible,
    },
};

use alloc::{boxed::Box, vec::Vec};
use core::marker::PhantomData;

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

    #[cfg_attr(feature = "std", error(transparent))]
    StorageError(StorageError),
}

impl<StorageError> From<StorageError> for MerkleTreeError<StorageError> {
    fn from(err: StorageError) -> MerkleTreeError<StorageError> {
        MerkleTreeError::StorageError(err)
    }
}

#[derive(Debug)]
pub struct MerkleTree<TableType, StorageType> {
    storage: StorageType,
    head: Option<Box<Subtree<Node>>>,
    leaves_count: u64,
    phantom_table: PhantomData<TableType>,
}

impl<TableType, StorageType, StorageError> MerkleTree<TableType, StorageType>
where
    TableType: Mappable<Key = u64, SetValue = Primitive, GetValue = Primitive>,
    StorageType: StorageInspect<TableType, Error = StorageError>,
{
    pub fn new(storage: StorageType) -> Self {
        Self {
            storage,
            head: None,
            leaves_count: 0,
            phantom_table: Default::default(),
        }
    }

    pub fn load(
        storage: StorageType,
        leaves_count: u64,
    ) -> Result<Self, MerkleTreeError<StorageError>> {
        let mut tree = Self {
            storage,
            head: None,
            leaves_count,
            phantom_table: Default::default(),
        };

        tree.build()?;

        Ok(tree)
    }

    pub fn root(&self) -> Bytes32 {
        let mut scratch_storage = StorageMap::<NodesTable>::new();
        let root_node = self.root_node(&mut scratch_storage);
        match root_node {
            None => *empty_sum(),
            Some(ref node) => *node.hash(),
        }
    }

    pub fn prove(
        &self,
        proof_index: u64,
    ) -> Result<(Bytes32, ProofSet), MerkleTreeError<StorageError>> {
        if proof_index + 1 > self.leaves_count {
            return Err(MerkleTreeError::InvalidProofIndex(proof_index));
        }

        let mut proof_set = ProofSet::new();

        let root_position = self.root_position();
        let leaf_position = Position::from_leaf_index(proof_index);
        let primitive = self
            .storage
            .get(&leaf_position.in_order_index())?
            .ok_or(MerkleTreeError::LoadError(proof_index))?
            .into_owned();
        let leaf_node = Node::from(primitive);
        proof_set.push(*leaf_node.hash());

        let (_, mut side_positions): (Vec<_>, Vec<_>) = root_position
            .path(&leaf_position, self.leaves_count)
            .iter()
            .unzip();
        side_positions.reverse(); // Reorder side positions from leaf to root.
        side_positions.pop(); // The last side position is the root; remove it.

        // Allocate scratch storage to store temporary nodes when building the
        // root.
        let mut scratch_storage = StorageMap::<NodesTable>::new();
        let root_node = self
            .root_node(&mut scratch_storage)
            .expect("Root node must be present");

        // Get side nodes. First, we check the scratch storage. If the side node
        // is not found in scratch storage, we then check main storage. Finally,
        // if the side node is not found in main storage, we exit with a load
        // error.
        for side_position in side_positions {
            let key = side_position.in_order_index();
            let primitive = StorageInspectInfallible::get(&scratch_storage, &key)
                .or(StorageInspect::get(&self.storage, &key)?)
                .ok_or(MerkleTreeError::LoadError(key))?
                .into_owned();
            let node = Node::from(primitive);
            proof_set.push(*node.hash());
        }

        let root = *root_node.hash();
        Ok((root, proof_set))
    }

    //
    // PRIVATE
    //

    /// A binary Merkle tree can be built from a collection of Merkle Mountain
    /// Range (MMR) peaks. The MMR structure can be accurately defined by the
    /// number of leaves in the leaf row.
    ///
    /// Consider a binary Merkle tree with seven leaves, producing the following
    /// MMR structure:
    ///
    /// ```text
    ///       03
    ///      /  \
    ///     /    \
    ///   01      05      09
    ///  /  \    /  \    /  \
    /// 00  02  04  06  08  10  12
    /// ```
    ///
    /// We observe that the tree has three peaks at positions `03`, `09`, and
    /// `12`. These peak positions are recorded in the order that they appear,
    /// reading left to right in the tree structure, and only descend in height.
    /// These peak positions communicate everything needed to determine the
    /// remaining internal nodes building upwards to the root position:
    ///
    /// ```text
    ///            07
    ///           /  \
    ///          /    \
    ///         /      \
    ///        /        \
    ///       /          \
    ///      /            \
    ///    03              11
    ///   /  \            /  \
    /// ...  ...         /    \
    ///                09      \
    ///               /  \      \
    ///             ...  ...    12
    /// ```
    ///
    /// No additional intermediate nodes or leaves are required to calculate
    /// the root position.
    ///
    /// The positions of the MMR peaks can be deterministically calculated as a
    /// function of `n + 1` where `n` is the number of leaves in the tree. By
    /// appending an additional leaf node to the tree, we generate a new tree
    /// structure with additional internal nodes (N.B.: this may also change the
    /// root position if the tree is already balanced).
    ///
    /// In our example, we add an additional leaf at leaf index `7` (in-order
    /// index `14`):
    ///
    /// ```text
    ///            07
    ///           /  \
    ///          /    \
    ///         /      \
    ///        /        \
    ///       /          \
    ///      /            \
    ///    03              11
    ///   /  \            /  \
    /// ...  ...         /    \
    ///                09      13
    ///               /  \    /  \
    ///             ...  ... 12  14
    /// ```
    ///
    /// We observe that the path from the root position to our new leaf position
    /// yields a set of side positions that includes our original peak
    /// positions (see [Path Iterator](crate::common::path_iterator::PathIter)):
    ///
    /// | Path position | Side position |
    /// |---------------|---------------|
    /// |            07 |            07 |
    /// |            11 |            03 |
    /// |            13 |            09 |
    /// |            14 |            12 |
    ///
    /// By excluding the root position `07`, we have established the set of
    /// side positions `03`, `09`, and `12`, matching our set of MMR peaks.
    ///
    fn build(&mut self) -> Result<(), MerkleTreeError<StorageError>> {
        let mut current_head = None;
        let peaks = &self.peak_positions();
        for peak in peaks.iter() {
            let key = peak.in_order_index();
            let node = self
                .storage
                .get(&key)?
                .ok_or(MerkleTreeError::LoadError(key))?
                .into_owned()
                .into();
            let next = Box::new(Subtree::<Node>::new(node, current_head));
            current_head = Some(next);
        }

        self.head = current_head;

        Ok(())
    }

    fn peak_positions(&self) -> Vec<Position> {
        // Define a new tree with a leaf count 1 greater than the current leaf
        // count.
        let leaves_count = self.leaves_count + 1;

        // The rightmost leaf position of a tree will always have a leaf index
        // N - 1, where N is the number of leaves.
        let leaf_position = Position::from_leaf_index(leaves_count - 1);
        let root_position = self.root_position();
        let mut peaks_itr = root_position.path(&leaf_position, leaves_count).iter();
        peaks_itr.next(); // Omit the root

        let (_, peaks): (Vec<_>, Vec<_>) = peaks_itr.unzip();

        peaks
    }

    fn root_position(&self) -> Position {
        // Define a new tree with a leaf count 1 greater than the current leaf
        // count.
        let leaves_count = self.leaves_count + 1;

        // The root position of a tree will always have an in-order index equal
        // to N' - 1, where N is the leaves count and N' is N rounded (or equal)
        // to the next power of 2.
        let root_index = leaves_count.next_power_of_two() - 1;
        Position::from_in_order_index(root_index)
    }

    /// The root node is generated by joining all MMR peaks, where a peak is
    /// defined as the head of a balanced subtree. A tree can be composed of a
    /// single balanced subtree, in which case the tree is itself balanced, or
    /// several balanced subtrees, in which case the tree is imbalanced. Only
    /// nodes at the head of a balanced tree are persisted in storage; any node,
    /// including the root node, whose child is an imbalanced child subtree will
    /// not be saved in persistent storage. This is because node data for such
    /// nodes is liable to change as more leaves are pushed to the tree.
    /// Instead, intermediate nodes must be held in a temporary storage space.
    ///
    /// When calling `root_node`, callees must pass a mutable reference to a
    /// temporary storage space that will be used to hold any intermediate nodes
    /// that are created during root node calculation. At the end of the method
    /// call, this temporary storage space will contain all intermediate nodes
    /// not held in persistent storage, and these nodes will be available to the
    /// callee.
    ///
    fn root_node(&self, scratch_storage: &mut StorageMap<NodesTable>) -> Option<Node> {
        self.head
            .as_ref()
            .map(|head| build_root_node(head, scratch_storage))
    }
}

impl<TableType, StorageType, StorageError> MerkleTree<TableType, StorageType>
where
    TableType: Mappable<Key = u64, SetValue = Primitive, GetValue = Primitive>,
    StorageType: StorageMutate<TableType, Error = StorageError>,
{
    pub fn push(&mut self, data: &[u8]) -> Result<(), MerkleTreeError<StorageError>> {
        let node = Node::create_leaf(self.leaves_count, data);
        self.storage.insert(&node.key(), &node.as_ref().into())?;
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
                let joined_head = join_subtrees(&mut head_next, &mut head);
                self.storage.insert(
                    &joined_head.node().key(),
                    &joined_head.node().as_ref().into(),
                )?;
                joined_head
            };
            self.head = Some(Box::new(joined_head));
        }

        Ok(())
    }
}

fn join_subtrees(lhs: &mut Subtree<Node>, rhs: &mut Subtree<Node>) -> Subtree<Node> {
    let joined_node = Node::create_node(lhs.node(), rhs.node());
    Subtree::new(joined_node, lhs.take_next())
}

fn build_root_node<Table, Storage>(subtree: &Subtree<Node>, storage: &mut Storage) -> Node
where
    Table: Mappable<Key = u64, GetValue = Primitive, SetValue = Primitive>,
    Storage: StorageMutateInfallible<Table>,
{
    let mut current = subtree.clone();
    while current.next().is_some() {
        let mut head = current;
        let mut head_next = head.take_next().unwrap();
        current = join_subtrees(&mut head_next, &mut head);
        storage.insert(&current.node().key(), &current.node().as_ref().into());
    }
    current.node().clone()
}

#[cfg(test)]
mod test {
    use super::{MerkleTree, MerkleTreeError};
    use crate::{
        binary::{empty_sum, leaf_sum, node_sum, Node, Primitive},
        common::StorageMap,
    };
    use fuel_merkle_test_helpers::TEST_DATA;
    use fuel_storage::{Mappable, StorageInspect};

    use alloc::vec::Vec;

    #[derive(Debug)]
    struct TestTable;

    impl Mappable for TestTable {
        type Key = u64;
        type SetValue = Primitive;
        type GetValue = Self::SetValue;
    }

    #[test]
    fn test_push_builds_internal_tree_structure() {
        let mut storage_map = StorageMap::<TestTable>::new();
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

        let leaf_0 = leaf_sum(data[0]);
        let leaf_1 = leaf_sum(data[1]);
        let leaf_2 = leaf_sum(data[2]);
        let leaf_3 = leaf_sum(data[3]);
        let leaf_4 = leaf_sum(data[4]);
        let leaf_5 = leaf_sum(data[5]);
        let leaf_6 = leaf_sum(data[6]);
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

        assert_eq!(*Node::from(s_leaf_0.into_owned()).hash(), leaf_0);
        assert_eq!(*Node::from(s_leaf_1.into_owned()).hash(), leaf_1);
        assert_eq!(*Node::from(s_leaf_2.into_owned()).hash(), leaf_2);
        assert_eq!(*Node::from(s_leaf_3.into_owned()).hash(), leaf_3);
        assert_eq!(*Node::from(s_leaf_4.into_owned()).hash(), leaf_4);
        assert_eq!(*Node::from(s_leaf_5.into_owned()).hash(), leaf_5);
        assert_eq!(*Node::from(s_leaf_6.into_owned()).hash(), leaf_6);
        assert_eq!(*Node::from(s_node_1.into_owned()).hash(), node_1);
        assert_eq!(*Node::from(s_node_5.into_owned()).hash(), node_5);
        assert_eq!(*Node::from(s_node_9.into_owned()).hash(), node_9);
        assert_eq!(*Node::from(s_node_3.into_owned()).hash(), node_3);
    }

    #[test]
    fn load_returns_a_valid_tree() {
        const LEAVES_COUNT: u64 = 2u64.pow(16) - 1;

        let mut storage_map = StorageMap::<TestTable>::new();

        let expected_root = {
            let mut tree = MerkleTree::new(&mut storage_map);
            let data = (0u64..LEAVES_COUNT)
                .map(|i| i.to_be_bytes())
                .collect::<Vec<_>>();
            for datum in data.iter() {
                let _ = tree.push(datum);
            }
            tree.root()
        };

        let root = {
            let tree = MerkleTree::load(&mut storage_map, LEAVES_COUNT).unwrap();
            tree.root()
        };

        assert_eq!(expected_root, root);
    }

    #[test]
    fn load_returns_empty_tree_for_0_leaves() {
        const LEAVES_COUNT: u64 = 0;

        let expected_root = *empty_sum();

        let root = {
            let mut storage_map = StorageMap::<TestTable>::new();
            let tree = MerkleTree::load(&mut storage_map, LEAVES_COUNT).unwrap();
            tree.root()
        };

        assert_eq!(expected_root, root);
    }

    #[test]
    fn load_returns_a_load_error_if_the_storage_is_not_valid_for_the_leaves_count() {
        const LEAVES_COUNT: u64 = 5;

        let mut storage_map = StorageMap::<TestTable>::new();

        let mut tree = MerkleTree::new(&mut storage_map);
        let data = (0u64..LEAVES_COUNT)
            .map(|i| i.to_be_bytes())
            .collect::<Vec<_>>();
        for datum in data.iter() {
            let _ = tree.push(datum);
        }

        let err = MerkleTree::load(&mut storage_map, LEAVES_COUNT * 2)
            .expect_err("Expected load() to return Error; got Ok");
        assert!(matches!(err, MerkleTreeError::LoadError(_)));
    }

    #[test]
    fn root_returns_the_empty_root_for_0_leaves() {
        let mut storage_map = StorageMap::<TestTable>::new();
        let tree = MerkleTree::new(&mut storage_map);

        let root = tree.root();
        assert_eq!(root, empty_sum().clone());
    }

    #[test]
    fn root_returns_the_merkle_root_for_1_leaf() {
        let mut storage_map = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let data = &TEST_DATA[0..1]; // 1 leaf
        for datum in data.iter() {
            let _ = tree.push(datum);
        }

        let leaf_0 = leaf_sum(data[0]);

        let root = tree.root();
        assert_eq!(root, leaf_0);
    }

    #[test]
    fn root_returns_the_merkle_root_for_7_leaves() {
        let mut storage_map = StorageMap::<TestTable>::new();
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

        let leaf_0 = leaf_sum(data[0]);
        let leaf_1 = leaf_sum(data[1]);
        let leaf_2 = leaf_sum(data[2]);
        let leaf_3 = leaf_sum(data[3]);
        let leaf_4 = leaf_sum(data[4]);
        let leaf_5 = leaf_sum(data[5]);
        let leaf_6 = leaf_sum(data[6]);

        let node_1 = node_sum(&leaf_0, &leaf_1);
        let node_5 = node_sum(&leaf_2, &leaf_3);
        let node_3 = node_sum(&node_1, &node_5);
        let node_9 = node_sum(&leaf_4, &leaf_5);
        let node_11 = node_sum(&node_9, &leaf_6);
        let node_7 = node_sum(&node_3, &node_11);

        let root = tree.root();
        assert_eq!(root, node_7);
    }

    #[test]
    fn prove_returns_invalid_proof_index_error_for_0_leaves() {
        let mut storage_map = StorageMap::<TestTable>::new();
        let tree = MerkleTree::new(&mut storage_map);

        let err = tree
            .prove(0)
            .expect_err("Expected prove() to return Error; got Ok");
        assert!(matches!(err, MerkleTreeError::InvalidProofIndex(0)));
    }

    #[test]
    fn prove_returns_invalid_proof_index_error_when_index_is_greater_than_number_of_leaves() {
        let mut storage_map = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let data = &TEST_DATA[0..5]; // 5 leaves
        for datum in data.iter() {
            let _ = tree.push(datum);
        }

        let err = tree
            .prove(10)
            .expect_err("Expected prove() to return Error; got Ok");
        assert!(matches!(err, MerkleTreeError::InvalidProofIndex(10)))
    }

    #[test]
    fn prove_returns_the_merkle_root_and_proof_set_for_1_leaf() {
        let mut storage_map = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let data = &TEST_DATA[0..1]; // 1 leaf
        for datum in data.iter() {
            let _ = tree.push(datum);
        }

        let leaf_0 = leaf_sum(data[0]);

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
        let mut storage_map = StorageMap::<TestTable>::new();
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

        let leaf_0 = leaf_sum(data[0]);
        let leaf_1 = leaf_sum(data[1]);
        let leaf_2 = leaf_sum(data[2]);
        let leaf_3 = leaf_sum(data[3]);

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
        let mut storage_map = StorageMap::<TestTable>::new();
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

        let leaf_0 = leaf_sum(data[0]);
        let leaf_1 = leaf_sum(data[1]);
        let leaf_2 = leaf_sum(data[2]);
        let leaf_3 = leaf_sum(data[3]);
        let leaf_4 = leaf_sum(data[4]);

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
        let mut storage_map = StorageMap::<TestTable>::new();
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

        let leaf_0 = leaf_sum(data[0]);
        let leaf_1 = leaf_sum(data[1]);
        let leaf_2 = leaf_sum(data[2]);
        let leaf_3 = leaf_sum(data[3]);
        let leaf_4 = leaf_sum(data[4]);
        let leaf_5 = leaf_sum(data[5]);
        let leaf_6 = leaf_sum(data[6]);

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

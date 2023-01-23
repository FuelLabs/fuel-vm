use crate::{
    common::{error::DeserializeError, AsPathIterator, Bytes32, ChildError},
    sparse::{primitive::Primitive, zero_sum, Node, StorageNode, StorageNodeError},
    storage::{Mappable, StorageMutate},
};

use alloc::{string::String, vec::Vec};
use core::{cmp, fmt, iter, marker::PhantomData};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum MerkleTreeError<StorageError> {
    #[cfg_attr(
        feature = "std",
        error("cannot load node with key {0}; the key is not found in storage")
    )]
    LoadError(String),

    #[cfg_attr(feature = "std", error(transparent))]
    StorageError(StorageError),

    #[cfg_attr(feature = "std", error(transparent))]
    DeserializeError(DeserializeError),

    #[cfg_attr(feature = "std", error(transparent))]
    ChildError(ChildError<Bytes32, StorageNodeError<StorageError>>),
}

impl<StorageError> From<StorageError> for MerkleTreeError<StorageError> {
    fn from(err: StorageError) -> MerkleTreeError<StorageError> {
        MerkleTreeError::StorageError(err)
    }
}

#[derive(Debug)]
pub struct MerkleTree<TableType, StorageType> {
    root_node: Node,
    storage: StorageType,
    phantom_table: PhantomData<TableType>,
}

impl<TableType, StorageType, StorageError> MerkleTree<TableType, StorageType>
where
    TableType: Mappable<Key<'static> = Bytes32, SetValue = Primitive, GetValue = Primitive>,
    StorageType: StorageMutate<TableType, Error = StorageError>,
    StorageError: fmt::Debug + Clone + 'static,
{
    pub fn new(storage: StorageType) -> Self {
        Self {
            root_node: Node::create_placeholder(),
            storage,
            phantom_table: Default::default(),
        }
    }

    pub fn load(storage: StorageType, root: &Bytes32) -> Result<Self, MerkleTreeError<StorageError>> {
        let primitive = storage
            .get(root)?
            .ok_or_else(|| MerkleTreeError::LoadError(hex::encode(root)))?
            .into_owned();
        let tree = Self {
            root_node: primitive.try_into().map_err(MerkleTreeError::DeserializeError)?,
            storage,
            phantom_table: Default::default(),
        };
        Ok(tree)
    }

    pub fn update(&mut self, key: &Bytes32, data: &[u8]) -> Result<(), MerkleTreeError<StorageError>> {
        if data.is_empty() {
            // If the data is empty, this signifies a delete operation for the
            // given key.
            self.delete(key)?;
            return Ok(());
        }

        let leaf_node = Node::create_leaf(key, data);
        self.storage.insert(&leaf_node.hash(), &leaf_node.as_ref().into())?;
        self.storage.insert(leaf_node.leaf_key(), &leaf_node.as_ref().into())?;

        if self.root_node().is_placeholder() {
            self.set_root_node(leaf_node);
        } else {
            let (path_nodes, side_nodes) = self.path_set(leaf_node.clone())?;
            self.update_with_path_set(&leaf_node, path_nodes.as_slice(), side_nodes.as_slice())?;
        }

        Ok(())
    }

    pub fn delete(&mut self, key: &Bytes32) -> Result<(), MerkleTreeError<StorageError>> {
        if self.root() == *zero_sum() {
            // The zero root signifies that all leaves are empty, including the
            // given key.
            return Ok(());
        }

        if let Some(primitive) = self.storage.get(key)? {
            let primitive = primitive.into_owned();
            let leaf_node: Node = primitive.try_into().map_err(MerkleTreeError::DeserializeError)?;
            let (path_nodes, side_nodes): (Vec<Node>, Vec<Node>) = self.path_set(leaf_node.clone())?;
            self.delete_with_path_set(&leaf_node, path_nodes.as_slice(), side_nodes.as_slice())?;
        }

        Ok(())
    }

    pub fn root(&self) -> Bytes32 {
        self.root_node().hash()
    }

    // PRIVATE

    fn root_node(&self) -> &Node {
        &self.root_node
    }

    fn set_root_node(&mut self, node: Node) {
        debug_assert!(node.is_leaf() || node.height() == Node::max_height() as u32);
        self.root_node = node;
    }

    fn path_set(&self, leaf_node: Node) -> Result<(Vec<Node>, Vec<Node>), MerkleTreeError<StorageError>> {
        let root_node = self.root_node().clone();
        let root_storage_node = StorageNode::new(&self.storage, root_node);
        let leaf_storage_node = StorageNode::new(&self.storage, leaf_node);
        let (mut path_nodes, mut side_nodes): (Vec<Node>, Vec<Node>) = root_storage_node
            .as_path_iter(&leaf_storage_node)
            .map(|(path_node, side_node)| {
                Ok((
                    path_node.map_err(MerkleTreeError::ChildError)?.into_node(),
                    side_node.map_err(MerkleTreeError::ChildError)?.into_node(),
                ))
            })
            .collect::<Result<Vec<_>, MerkleTreeError<StorageError>>>()?
            .into_iter()
            .unzip();
        path_nodes.reverse();
        side_nodes.reverse();
        side_nodes.pop(); // The last element in the side nodes list is the
                          // root; remove it.

        Ok((path_nodes, side_nodes))
    }

    fn update_with_path_set(
        &mut self,
        requested_leaf_node: &Node,
        path_nodes: &[Node],
        side_nodes: &[Node],
    ) -> Result<(), StorageError> {
        let path = requested_leaf_node.leaf_key();
        let actual_leaf_node = &path_nodes[0];

        // Build the tree upwards starting with the requested leaf node.
        let mut current_node = requested_leaf_node.clone();

        // If we are creating a new leaf node, the corresponding side node will
        // be the first node in the path set. The side node will be the leaf
        // node currently closest to the requested new leaf node. When creating
        // a new leaf node, we must merge the leaf node with its corresponding
        // side node to create a common ancestor. We then continue building the
        // tree upwards from this ancestor node. This may require creating new
        // placeholder side nodes, in addition to the existing side node set.
        //
        // If we are updating an existing leaf node, the leaf node we are
        // updating is the first node in the path set. The side node set will
        // already include all the side nodes needed to build up the tree from
        // the requested leaf node, since these side nodes were already built
        // during the creation of the leaf node.
        //
        // We can determine if we are updating an existing leaf node, or if we
        // are creating a new leaf node, by comparing the paths of the requested
        // leaf node and the leaf node at the start of the path set. When the
        // paths are equal, it means the leaf nodes occupy the same location,
        // and we are updating an existing leaf. Otherwise, it means we are
        // adding a new leaf node.
        if requested_leaf_node.leaf_key() != actual_leaf_node.leaf_key() {
            // Merge leaves
            if !actual_leaf_node.is_placeholder() {
                current_node = Node::create_node_on_path(path, &current_node, actual_leaf_node);
                self.storage
                    .insert(&current_node.hash(), &current_node.as_ref().into())?;
            }

            // Merge placeholders
            let ancestor_depth = requested_leaf_node.common_path_length(actual_leaf_node);
            let stale_depth = cmp::max(side_nodes.len(), ancestor_depth);
            let placeholders_count = stale_depth - side_nodes.len();
            let placeholders = iter::repeat(Node::create_placeholder()).take(placeholders_count);
            for placeholder in placeholders {
                current_node = Node::create_node_on_path(path, &current_node, &placeholder);
                self.storage
                    .insert(&current_node.hash(), &current_node.as_ref().into())?;
            }
        }

        // Merge side nodes
        for side_node in side_nodes {
            current_node = Node::create_node_on_path(path, &current_node, side_node);
            self.storage
                .insert(&current_node.hash(), &current_node.as_ref().into())?;
        }

        self.set_root_node(current_node);

        Ok(())
    }

    fn delete_with_path_set(
        &mut self,
        requested_leaf_node: &Node,
        path_nodes: &[Node],
        side_nodes: &[Node],
    ) -> Result<(), StorageError> {
        for node in path_nodes {
            self.storage.remove(&node.hash())?;
        }

        let path = requested_leaf_node.leaf_key();
        let mut side_nodes_iter = side_nodes.iter();

        // The deleted leaf is replaced by a placeholder. Build the tree upwards
        // starting with the placeholder.
        let mut current_node = Node::create_placeholder();

        // If the first side node is a leaf, it means the ancestor node is now
        // parent to a placeholder (the deleted leaf node) and a leaf node (the
        // first side node). We can immediately discard the ancestor node from
        // further calculation and attach the orphaned leaf node to its next
        // ancestor. Any subsequent ancestor nodes composed of this leaf node
        // and a placeholder must be similarly discarded from further
        // calculation. We then create a valid ancestor node for the orphaned
        // leaf node by joining it with the earliest non-placeholder side node.
        if let Some(first_side_node) = side_nodes.first() {
            if first_side_node.is_leaf() {
                side_nodes_iter.next();
                current_node = first_side_node.clone();

                // Advance the side node iterator to the next non-placeholder
                // node. This may be either another leaf node or an internal
                // node. If only placeholder nodes exist beyond the first leaf
                // node, then that leaf node is, in fact, the new root node.
                //
                // Using `find(..)` advances the iterator beyond the next
                // non-placeholder side node and returns it. Therefore, we must
                // consume the side node at this point. If another non-
                // placeholder node was found in the side node collection, merge
                // it with the first side node. This guarantees that the current
                // node will be an internal node, and not a leaf, by the time we
                // start merging the remaining side nodes.
                // See https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.find.
                if let Some(side_node) = side_nodes_iter.find(|side_node| !side_node.is_placeholder()) {
                    current_node = Node::create_node_on_path(path, &current_node, side_node);
                    self.storage
                        .insert(&current_node.hash(), &current_node.as_ref().into())?;
                }
            }
        }

        // Merge side nodes
        for side_node in side_nodes_iter {
            current_node = Node::create_node_on_path(path, &current_node, side_node);
            self.storage
                .insert(&current_node.hash(), &current_node.as_ref().into())?;
        }

        self.set_root_node(current_node);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        common::{Bytes32, StorageMap},
        sparse::{hash::sum, MerkleTree, MerkleTreeError, Primitive},
    };
    use fuel_storage::Mappable;
    use hex;

    #[derive(Debug)]
    struct TestTable;

    impl Mappable for TestTable {
        type Key<'a> = Bytes32;
        type SetValue = Primitive;
        type GetValue = Self::SetValue;
    }

    #[test]
    fn test_empty_root() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let tree = MerkleTree::new(&mut storage);
        let root = tree.root();
        let expected_root = "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_1() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();

        let root = tree.root();
        let expected_root = "39f36a7cb4dfb1b46f03d044265df6a491dffc1034121bc1071a34ddce9bb14b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_2() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x01"), b"DATA").unwrap();

        let root = tree.root();
        let expected_root = "8d0ae412ca9ca0afcb3217af8bcd5a673e798bd6fd1dfacad17711e883f494cb";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_3() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x01"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x02"), b"DATA").unwrap();

        let root = tree.root();
        let expected_root = "52295e42d8de2505fdc0cc825ff9fead419cbcf540d8b30c7c4b9c9b94c268b7";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_5() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x01"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x02"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x03"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x04"), b"DATA").unwrap();

        let root = tree.root();
        let expected_root = "108f731f2414e33ae57e584dc26bd276db07874436b2264ca6e520c658185c6b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_10() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..10 {
            let key = sum(i.to_be_bytes());
            tree.update(&key, b"DATA").unwrap();
        }

        let root = tree.root();
        let expected_root = "21ca4917e99da99a61de93deaf88c400d4c082991cb95779e444d43dd13e8849";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_100() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..100 {
            let key = sum(i.to_be_bytes());
            tree.update(&key, b"DATA").unwrap();
        }

        let root = tree.root();
        let expected_root = "82bf747d455a55e2f7044a03536fc43f1f55d43b855e72c0110c986707a23e4d";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_with_repeated_inputs() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();

        let root = tree.root();
        let expected_root = "39f36a7cb4dfb1b46f03d044265df6a491dffc1034121bc1071a34ddce9bb14b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_overwrite_key() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x00"), b"CHANGE").unwrap();

        let root = tree.root();
        let expected_root = "dd97174c80e5e5aa3a31c61b05e279c1495c8a07b2a08bca5dbc9fb9774f9457";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_union() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..5 {
            let key = sum(i.to_be_bytes());
            tree.update(&key, b"DATA").unwrap();
        }

        for i in 10_u32..15 {
            let key = sum(i.to_be_bytes());
            tree.update(&key, b"DATA").unwrap();
        }

        for i in 20_u32..25 {
            let key = sum(i.to_be_bytes());
            tree.update(&key, b"DATA").unwrap();
        }

        let root = tree.root();
        let expected_root = "7e6643325042cfe0fc76626c043b97062af51c7e9fc56665f12b479034bce326";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_sparse_union() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x02"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x04"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x06"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x08"), b"DATA").unwrap();

        let root = tree.root();
        let expected_root = "e912e97abc67707b2e6027338292943b53d01a7fbd7b244674128c7e468dd696";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_with_empty_data() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(&sum(b"\x00\x00\x00\x00"), b"").unwrap();

        let root = tree.root();
        let expected_root = "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_with_empty_performs_delete() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x00"), b"").unwrap();

        let root = tree.root();
        let expected_root = "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_1_delete_1() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.delete(&sum(b"\x00\x00\x00\x00")).unwrap();

        let root = tree.root();
        let expected_root = "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_2_delete_1() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x01"), b"DATA").unwrap();
        tree.delete(&sum(b"\x00\x00\x00\x01")).unwrap();

        let root = tree.root();
        let expected_root = "39f36a7cb4dfb1b46f03d044265df6a491dffc1034121bc1071a34ddce9bb14b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_10_delete_5() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..10 {
            let key = sum(i.to_be_bytes());
            tree.update(&key, b"DATA").unwrap();
        }

        for i in 5_u32..10 {
            let key = sum(i.to_be_bytes());
            tree.delete(&key).unwrap();
        }

        let root = tree.root();
        let expected_root = "108f731f2414e33ae57e584dc26bd276db07874436b2264ca6e520c658185c6b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_delete_non_existent_key() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x01"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x02"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x03"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x04"), b"DATA").unwrap();
        tree.delete(&sum(b"\x00\x00\x04\x00")).unwrap();

        let root = tree.root();
        let expected_root = "108f731f2414e33ae57e584dc26bd276db07874436b2264ca6e520c658185c6b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_interleaved_update_delete() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..10 {
            let key = sum(i.to_be_bytes());
            tree.update(&key, b"DATA").unwrap();
        }

        for i in 5_u32..15 {
            let key = sum(i.to_be_bytes());
            tree.delete(&key).unwrap();
        }

        for i in 10_u32..20 {
            let key = sum(i.to_be_bytes());
            tree.update(&key, b"DATA").unwrap();
        }

        for i in 15_u32..25 {
            let key = sum(i.to_be_bytes());
            tree.delete(&key).unwrap();
        }

        for i in 20_u32..30 {
            let key = sum(i.to_be_bytes());
            tree.update(&key, b"DATA").unwrap();
        }

        for i in 25_u32..35 {
            let key = sum(i.to_be_bytes());
            tree.delete(&key).unwrap();
        }

        let root = tree.root();
        let expected_root = "7e6643325042cfe0fc76626c043b97062af51c7e9fc56665f12b479034bce326";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_delete_sparse_union() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..10 {
            let key = sum(i.to_be_bytes());
            tree.update(&key, b"DATA").unwrap();
        }

        for i in 0_u32..5 {
            let key = sum((i * 2 + 1).to_be_bytes());
            tree.delete(&key).unwrap();
        }

        let root = tree.root();
        let expected_root = "e912e97abc67707b2e6027338292943b53d01a7fbd7b244674128c7e468dd696";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_load_returns_a_valid_tree() {
        // Instantiate a new key-value storage backing and populate it using a sparse
        // Merkle tree. The root of the Merkle tree is the key that maps to the buffer
        // of the root node in the storage. When loading a Merkle tree from storage, we
        // need a reference to the storage object, as well as the root that allows us to
        // look up the buffer of the root node. We will later use this storage backing
        // and root to load a Merkle tree.
        let (mut storage_to_load, root_to_load) = {
            let mut storage = StorageMap::<TestTable, Bytes32>::new();
            let mut tree = MerkleTree::new(&mut storage);
            tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x01"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x02"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x03"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x04"), b"DATA").unwrap();
            let root = tree.root();
            (storage, root)
        };

        // Generate an expected root for this test by using both the set of `update`
        // data used when generating the loadable storage above and an additional set of
        // `update` data.
        let expected_root = {
            let mut storage = StorageMap::<TestTable, Bytes32>::new();
            let mut tree = MerkleTree::new(&mut storage);
            tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x01"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x02"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x03"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x04"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x05"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x06"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x07"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x08"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x09"), b"DATA").unwrap();
            tree.root()
        };

        let root = {
            // Create a Merkle tree by loading the generated storage and root.
            let mut tree = MerkleTree::load(&mut storage_to_load, &root_to_load).unwrap();
            // Build up the loaded tree using the additional set of `update` data so its
            // root matches the expected root. This verifies that the loaded tree has
            // successfully wrapped the given storage backing and assumed the correct state
            // so that future updates can be made seamlessly.
            tree.update(&sum(b"\x00\x00\x00\x05"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x06"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x07"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x08"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x09"), b"DATA").unwrap();
            tree.root()
        };

        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_load_returns_a_load_error_if_the_storage_is_not_valid_for_the_root() {
        let mut storage = StorageMap::<TestTable, Bytes32>::new();

        {
            let mut tree = MerkleTree::new(&mut storage);
            tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x01"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x02"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x03"), b"DATA").unwrap();
            tree.update(&sum(b"\x00\x00\x00\x04"), b"DATA").unwrap();
        }

        let root = &sum(b"\xff\xff\xff\xff");
        let err = MerkleTree::load(&mut storage, root).expect_err("Expected load() to return Error; got Ok");
        assert!(matches!(err, MerkleTreeError::LoadError(_)));
    }

    #[test]
    fn test_load_returns_a_deserialize_error_if_the_storage_is_corrupted() {
        use fuel_storage::StorageMutate;

        let mut storage = StorageMap::<TestTable, Bytes32>::new();

        let mut tree = MerkleTree::new(&mut storage);
        tree.update(&sum(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x01"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x02"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x03"), b"DATA").unwrap();
        tree.update(&sum(b"\x00\x00\x00\x04"), b"DATA").unwrap();
        let root = tree.root();

        // Overwrite the root key-value with an invalid primitive to create a
        // DeserializeError.
        let primitive = (0xff, 0xff, [0xff; 32], [0xff; 32]);
        storage.insert(&root, &primitive).unwrap();

        let err = MerkleTree::load(&mut storage, &root).expect_err("Expected load() to return Error; got Ok");
        assert!(matches!(err, MerkleTreeError::DeserializeError(_)));
    }
}

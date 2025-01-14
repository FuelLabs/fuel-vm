use fuel_storage::{
    StorageAsMut as _,
    StorageAsRef,
    StorageInspect,
    StorageMutate,
};
use jmt::storage;

use crate::jellyfish::merkle_tree::MerkleTreeStorage;

use crate::{
    common::{
        Bytes32,
        ProofSet,
        StorageMap,
    },
    jellyfish::{
        self,
        Primitive,
    },
    sparse::MerkleTreeKey,
    storage::Mappable,
};

use alloc::borrow::Cow;

/// The table of the JellyFish Merkle Tree's nodes.
#[derive(Debug, Clone)]
pub struct NodesTable;

impl Mappable for NodesTable {
    type Key = Self::OwnedKey;
    type OwnedKey = jmt::storage::NodeKey;
    type OwnedValue = jmt::storage::Node;
    type Value = Self::OwnedValue;
}
/// The table of the Binary Merkle Tree's values.
#[derive(Debug, Clone)]
pub struct ValuesTable;

impl Mappable for ValuesTable {
    type Key = Self::OwnedKey;
    type OwnedKey = jmt::KeyHash;
    type OwnedValue = (jmt::Version, jmt::OwnedValue);
    type Value = Self::OwnedValue;
}

#[derive(Debug, Clone)]

/// Auxiliary table that stores information about the rightmost leaf in the tree.
pub struct RightmostLeafTable;

impl Mappable for RightmostLeafTable {
    type Key = Self::OwnedKey;
    type OwnedKey = ();
    type OwnedValue = (jmt::KeyHash, jmt::storage::NodeKey);
    type Value = Self::OwnedValue;
}

#[derive(Debug, Clone)]

/// Auxiliary table that stores the latest version of the tree.
pub struct LatestRootVersionTable;

impl Mappable for LatestRootVersionTable {
    type Key = Self::OwnedKey;
    type OwnedKey = ();
    type OwnedValue = jmt::Version;
    type Value = Self::OwnedValue;
}

#[derive(Debug, Default, Clone)]
pub struct Storage {
    pub nodes: StorageMap<NodesTable>,
    pub values: StorageMap<ValuesTable>,
    pub rightmost_leaf: Option<<RightmostLeafTable as Mappable>::OwnedValue>,
    pub latest_root_version: Option<<LatestRootVersionTable as Mappable>::OwnedValue>,
}

impl StorageInspect<NodesTable> for Storage {
    type Error = core::convert::Infallible;

    fn get(
        &self,
        key: &<NodesTable as Mappable>::Key,
    ) -> Result<Option<std::borrow::Cow<<NodesTable as Mappable>::OwnedValue>>, Self::Error>
    {
        self.nodes.storage::<NodesTable>().get(key)
    }

    fn contains_key(
        &self,
        key: &<NodesTable as Mappable>::Key,
    ) -> Result<bool, Self::Error> {
        self.nodes.storage::<NodesTable>().contains_key(key)
    }
}

impl StorageInspect<ValuesTable> for Storage {
    type Error = core::convert::Infallible;

    fn get(
        &self,
        key: &<ValuesTable as Mappable>::Key,
    ) -> Result<
        Option<std::borrow::Cow<<ValuesTable as Mappable>::OwnedValue>>,
        Self::Error,
    > {
        self.values.storage::<ValuesTable>().get(key)
    }

    fn contains_key(
        &self,
        key: &<ValuesTable as Mappable>::Key,
    ) -> Result<bool, Self::Error> {
        self.values.storage::<ValuesTable>().contains_key(key)
    }
}

impl StorageInspect<RightmostLeafTable> for Storage {
    type Error = core::convert::Infallible;

    fn get(
        &self,
        _key: &<RightmostLeafTable as Mappable>::Key,
    ) -> Result<
        Option<std::borrow::Cow<<RightmostLeafTable as Mappable>::OwnedValue>>,
        Self::Error,
    > {
        let rightmost_leaf = &self.rightmost_leaf;
        if let Some(rightmost_leaf) = rightmost_leaf {
            Ok(Some(Cow::Borrowed(rightmost_leaf)))
        } else {
            Ok(None)
        }
    }

    fn contains_key(
        &self,
        _key: &<RightmostLeafTable as Mappable>::Key,
    ) -> Result<bool, Self::Error> {
        Ok(self.rightmost_leaf.is_some())
    }
}

impl StorageInspect<LatestRootVersionTable> for Storage {
    type Error = core::convert::Infallible;

    fn get(
        &self,
        _key: &<LatestRootVersionTable as Mappable>::Key,
    ) -> Result<
        Option<std::borrow::Cow<<LatestRootVersionTable as Mappable>::OwnedValue>>,
        Self::Error,
    > {
        let latest_root_version = &self.latest_root_version;
        if let Some(latest_root_version) = latest_root_version {
            Ok(Some(Cow::Borrowed(latest_root_version)))
        } else {
            Ok(None)
        }
    }

    fn contains_key(
        &self,
        _key: &<LatestRootVersionTable as Mappable>::Key,
    ) -> Result<bool, Self::Error> {
        Ok(self.latest_root_version.is_some())
    }
}

impl StorageMutate<NodesTable> for Storage {
    fn replace(
        &mut self,
        key: &<NodesTable as Mappable>::Key,
        value: &<NodesTable as Mappable>::Value,
    ) -> Result<Option<<NodesTable as Mappable>::OwnedValue>, Self::Error> {
        self.nodes.storage_as_mut().replace(key, value)
    }

    fn take(
        &mut self,
        key: &<NodesTable as Mappable>::Key,
    ) -> Result<Option<<NodesTable as Mappable>::OwnedValue>, Self::Error> {
        self.nodes.storage_as_mut().take(key)
    }
}

impl StorageMutate<ValuesTable> for Storage {
    fn replace(
        &mut self,
        key: &<ValuesTable as Mappable>::Key,
        value: &<ValuesTable as Mappable>::Value,
    ) -> Result<Option<<ValuesTable as Mappable>::OwnedValue>, Self::Error> {
        self.values.storage_as_mut().replace(key, value)
    }

    fn take(
        &mut self,
        key: &<ValuesTable as Mappable>::Key,
    ) -> Result<Option<<ValuesTable as Mappable>::OwnedValue>, Self::Error> {
        self.values.storage_as_mut().take(key)
    }
}

impl StorageMutate<RightmostLeafTable> for Storage {
    fn replace(
        &mut self,
        _key: &<RightmostLeafTable as Mappable>::Key,
        value: &<RightmostLeafTable as Mappable>::Value,
    ) -> Result<Option<<RightmostLeafTable as Mappable>::OwnedValue>, Self::Error> {
        let old_value = self.rightmost_leaf.take();
        self.rightmost_leaf = Some(value.clone());
        Ok(old_value)
    }

    fn take(
        &mut self,
        _key: &<RightmostLeafTable as Mappable>::Key,
    ) -> Result<Option<<RightmostLeafTable as Mappable>::OwnedValue>, Self::Error> {
        Ok(self.rightmost_leaf.take())
    }
}

impl StorageMutate<LatestRootVersionTable> for Storage {
    fn replace(
        &mut self,
        _key: &<LatestRootVersionTable as Mappable>::Key,
        value: &<LatestRootVersionTable as Mappable>::Value,
    ) -> Result<Option<<LatestRootVersionTable as Mappable>::OwnedValue>, Self::Error>
    {
        let old_value = self.latest_root_version.take();
        self.latest_root_version = Some(value.clone());
        Ok(old_value)
    }

    fn take(
        &mut self,
        _key: &<LatestRootVersionTable as Mappable>::Key,
    ) -> Result<Option<<LatestRootVersionTable as Mappable>::OwnedValue>, Self::Error>
    {
        Ok(self.latest_root_version.take())
    }
}

#[derive(Clone)]
pub struct MerkleTree {
    tree: MerkleTreeStorage<
        NodesTable,
        ValuesTable,
        RightmostLeafTable,
        LatestRootVersionTable,
        Storage,
    >,
}

impl MerkleTree {
    pub fn new() -> Self {
        let storage = Storage::default();
        Self {
            tree: MerkleTreeStorage::new(storage),
        }
    }

    /// Build a sparse Merkle tree from a set of key-value pairs. This is
    /// equivalent to creating an empty sparse Merkle tree and sequentially
    /// calling [update](Self::update) for each key-value pair. This constructor
    /// is more performant than calling individual sequential updates and is the
    /// preferred approach when the key-values are known upfront. Leaves can be
    /// appended to the returned tree using `update` to further accumulate leaf
    /// data.
    pub fn from_set<I, D>(set: I) -> Self
    where
        I: Iterator<Item = (MerkleTreeKey, D)>,
        D: AsRef<[u8]>,
    {
        let storage = Storage::default();
        let tree = MerkleTreeStorage::from_set(storage, set)
            .expect("`Storage` can't return error");
        Self { tree }
    }

    /// Calculate the sparse Merkle root from a set of key-value pairs. This is
    /// similar to constructing a new tree from a set of key-value pairs using
    /// [from_set](Self::from_set), except this method returns only the root; it
    /// does not write to storage nor return a sparse Merkle tree instance. It
    /// is equivalent to calling `from_set(..)`, followed by `root()`, but does
    /// not incur the overhead of storage writes. This can be helpful when we
    /// know all the key-values in the set upfront and we will not need to
    /// update the set in the future.
    pub fn root_from_set<I, D>(set: I) -> Bytes32
    where
        I: Iterator<Item = (MerkleTreeKey, D)>,
        D: AsRef<[u8]>,
    {
        let tree = Self::from_set(set);
        tree.root()
    }

    /// Calculate the sparse Merkle root as well as all nodes in the Merkle tree
    /// from a set of key-value pairs. This is similar to constructing a new
    /// tree from a set of key-value pairs using [from_set](Self::from_set),
    /// except this method returns only the root and the list of leaves and
    /// nodes in the tree; it does not return a sparse Merkle tree instance.
    /// This can be helpful when we know all the key-values in the set upfront
    /// and we need to defer storage writes, such as expensive database inserts,
    /// for batch operations later in the process.
    pub fn nodes_from_set<I, D>(
        set: I,
    ) -> (Bytes32, Vec<(jmt::storage::NibblePath, jmt::storage::Node)>)
    where
        I: Iterator<Item = (MerkleTreeKey, D)>,
        D: AsRef<[u8]>,
    {
        let tree = Self::from_set(set);
        let root = tree.root();
        let storage_read_guard = tree.tree.storage.read();
        let nodes = storage_read_guard
            .nodes
            .inner()
            .iter()
            .map(|(k, v)| (k.nibble_path().clone(), v.clone()))
            .collect();
        (root, nodes)
    }

    pub fn update(&mut self, key: MerkleTreeKey, data: &[u8]) {
        let _ = self.tree.update(key, data);
    }

    pub fn delete(&mut self, key: MerkleTreeKey) {
        let _ = self.tree.delete(key);
    }

    pub fn root(&self) -> Bytes32 {
        self.tree.root()
    }

    // pub fn generate_proof(&self, key: &MerkleTreeKey) -> Option<Proof> {
    //    self.tree.generate_proof(key).ok()
    //}
}

#[cfg(test)]
mod test {
    use crate::sparse::MerkleTreeKey;
    use jellyfish::hash::empty_sum;
    use sha2::Sha256;

    use super::*;

    #[test]
    fn root_returns_the_empty_root_for_0_leaves() {
        let tree = MerkleTree::new();

        let root = tree.root();
        assert_eq!(root, empty_sum().clone());
    }

    #[test]
    fn adding_key_value_pair_works() {
        let mut tree = MerkleTree::new();
        let raw_key = b"key";
        let merkle_tree_key = MerkleTreeKey::new(raw_key);
        let data = b"data";
        tree.update(merkle_tree_key, data);
        let storage = tree.tree.storage.read();
        let nodes = storage.nodes.inner();
        let values = storage.values.inner();
        // The version has been updated:
        assert_eq!(storage.latest_root_version.unwrap(), 1);
        // The rightmost leaf has been updated:
        assert!(storage.rightmost_leaf.is_some());
        // The root has been updated:
        assert!(&tree.root() != empty_sum());
        // There is exactly one node in the tree
        assert_eq!(nodes.len(), 1);
        // There is exactly one value in the tree
        assert_eq!(values.len(), 1);
        let node = nodes.iter().next().unwrap();
        // The only node is a leaf
        let jmt::storage::Node::Leaf(leaf_node) = node.1 else {
            panic!("Not a leaf node");
        };

        let value = values.iter().next().unwrap();
        let (value_key_hash, (_version, preimage)) = value;
        // The key_hash of the leaf node is the same as the key_hash of the value
        assert_eq!(value_key_hash, &leaf_node.key_hash());
        let preimage_value_hash = jmt::ValueHash::with::<Sha256>(preimage);
        let expected_leaf_node =
            jmt::storage::LeafNode::new(value_key_hash.clone(), preimage_value_hash);
        assert_eq!(leaf_node, &expected_leaf_node);
    }
}

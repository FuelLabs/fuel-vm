use fuel_storage::{
    StorageAsMut as _,
    StorageAsRef,
    StorageInspect,
    StorageMutate,
};

use crate::jellyfish::merkle_tree::MerkleTreeStorage;

use crate::{
    common::{
        Bytes32,
        StorageMap,
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
    tree: MerkleTreeStorage<NodesTable, ValuesTable, LatestRootVersionTable, Storage>,
}

impl MerkleTree {
    pub fn new() -> anyhow::Result<Self> {
        let storage = Storage::default();
        Ok(Self {
            tree: MerkleTreeStorage::new(storage)?,
        })
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
    pub fn root_from_set<I, D>(set: I) -> anyhow::Result<Bytes32>
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
    ) -> anyhow::Result<(Bytes32, Vec<(jmt::storage::NibblePath, jmt::storage::Node)>)>
    where
        I: Iterator<Item = (MerkleTreeKey, D)>,
        D: AsRef<[u8]>,
    {
        let tree = Self::from_set(set);
        let root = tree.root()?;
        let storage_read_guard = tree.tree.storage.read();
        let nodes = storage_read_guard
            .nodes
            .inner()
            .iter()
            .map(|(k, v)| (k.nibble_path().clone(), v.clone()))
            .collect();
        Ok((root, nodes))
    }

    pub fn update(&mut self, key: MerkleTreeKey, data: &[u8]) {
        let _ = self.tree.update(key, data);
    }

    pub fn delete(&mut self, key: MerkleTreeKey) {
        let _ = self.tree.delete(key);
    }

    pub fn root(&self) -> anyhow::Result<Bytes32> {
        self.tree.root()
    }

    // pub fn generate_proof(&self, key: &MerkleTreeKey) -> Option<Proof> {
    //    self.tree.generate_proof(key).ok()
    //}
}

#[cfg(test)]
mod test {
    use crate::{
        jellyfish::merkle_tree::EMPTY_ROOT,
        sparse::MerkleTreeKey,
    };
    use sha2::Sha256;

    use super::*;

    #[test]
    fn root_returns_the_empty_root_for_0_leaves() {
        let tree = MerkleTree::new().unwrap();

        let root = tree.root().unwrap();
        assert_eq!(root, EMPTY_ROOT);
    }

    #[test]
    fn adding_key_value_pair_works() {
        let mut tree = MerkleTree::new().unwrap();
        let initial_storage_version =
            tree.tree.storage.read().latest_root_version.unwrap();
        let raw_key = b"key";
        let merkle_tree_key = MerkleTreeKey::new(raw_key);
        let data = b"data";
        tree.update(merkle_tree_key, data);
        let storage = tree.tree.storage.read();
        let nodes = storage.nodes.inner();
        let values = storage.values.inner();
        // The version has been updated:
        assert_eq!(
            storage.latest_root_version.unwrap(),
            initial_storage_version + 1
        );
        // The root has been updated:
        assert_ne!(tree.root().unwrap(), EMPTY_ROOT);
        // There is exactly one value in the tree
        assert_eq!(values.len(), 1);
        let leaves = nodes
            .iter()
            .filter_map(|(_node_key, node)| match node {
                jmt::storage::Node::Leaf(leaf_node) => Some(leaf_node),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(leaves.len(), 1);
        let leaf_node = leaves[0];
        // The only node is a leaf

        let value = values.iter().next().unwrap();
        let (value_key_hash, (_version, preimage)) = value;
        // The key_hash of the leaf node is the same as the key_hash of the value
        assert_eq!(value_key_hash, &leaf_node.key_hash());
        let preimage_value_hash = jmt::ValueHash::with::<Sha256>(preimage);
        let expected_leaf_node =
            jmt::storage::LeafNode::new(value_key_hash.clone(), preimage_value_hash);
        assert_eq!(leaf_node, &expected_leaf_node);
    }

    #[test]
    fn adding_and_removing_key_value_pair_gives_the_empty_root() {
        let mut tree = MerkleTree::new().unwrap();
        let raw_key = b"key";
        let merkle_tree_key = MerkleTreeKey::new(raw_key);
        let data = b"data";
        tree.update(merkle_tree_key, data);
        tree.delete(merkle_tree_key);
        let first_root = tree.root().unwrap();
        assert_eq!(first_root, EMPTY_ROOT);
        tree.update(merkle_tree_key, data);
        tree.delete(merkle_tree_key);
        println!("{:?}", tree.tree.storage.read());
        let second_root = tree.root().unwrap();
        assert_eq!(second_root, EMPTY_ROOT);
    }

    #[test]
    fn updating_key_with_same_value_does_not_change_root() {
        let mut tree = MerkleTree::new().unwrap();
        let raw_key = b"key";
        let merkle_tree_key = MerkleTreeKey::new(raw_key);
        let data = b"data";
        tree.update(merkle_tree_key, data);
        let first_root = tree.root().unwrap();
        tree.update(merkle_tree_key, data);
        let second_root = tree.root().unwrap();
        assert_eq!(first_root, second_root);
    }

    #[test]
    fn updating_same_key_changes_root() {
        let mut tree = MerkleTree::new().unwrap();
        let raw_key = b"key";
        let merkle_tree_key = MerkleTreeKey::new(raw_key);
        let data1 = b"data1";
        let data2 = b"data2";
        tree.update(merkle_tree_key, data1);
        let first_root = tree.root().unwrap();
        tree.update(merkle_tree_key, data2);
        let second_root = tree.root().unwrap();
        assert_ne!(first_root, second_root);
    }

    #[test]
    fn verify_exclusion_proof_for_empty_tree_succeeds() {
        let tree = MerkleTree::new().unwrap();
        let raw_key = b"key";
        let merkle_tree_key = MerkleTreeKey::new(raw_key);
        let proof = tree.tree.generate_proof(&merkle_tree_key).unwrap();
        assert!(proof.verify(EMPTY_ROOT));
    }

    #[test]
    fn verify_inclusion_proof_succeeds() {
        let mut tree = MerkleTree::new().unwrap();
        let raw_key = b"key";
        let merkle_tree_key = MerkleTreeKey::new(raw_key);
        let data = b"data";
        tree.update(merkle_tree_key, data);
        let root = tree.root().unwrap();
        let proof = tree.tree.generate_proof(&merkle_tree_key).unwrap();
        assert!(proof.verify(root));
    }
}

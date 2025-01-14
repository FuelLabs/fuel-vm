use crate::{
    common::Bytes32,
    sparse::MerkleTreeKey,
    storage::{
        Mappable,
        StorageInspect,
        StorageMutate,
    },
};

use alloc::{
    sync::Arc,
    vec::Vec,
};
use core::{
    marker::PhantomData,
    sync::atomic::AtomicU64,
};
use spin::rwlock::RwLock;

use jmt::{
    storage::{
        HasPreimage,
        NodeKey,
        TreeReader,
        TreeWriter,
    },
    JellyfishMerkleTree,
    Sha256Jmt,
};

#[derive(Debug, Clone, derive_more::Display, PartialEq, Eq)]
pub enum MerkleTreeError<StorageError> {
    #[display(fmt = "proof index {_0} is not valid")]
    InvalidProofIndex(u64),

    #[display(fmt = "cannot load node with key {_0}; the key is not found in storage")]
    LoadError(u64),

    #[display(fmt = "{}", _0)]
    StorageError(StorageError),

    #[display(fmt = "the tree is too large")]
    TooLarge,
}

impl<StorageError> From<StorageError> for MerkleTreeError<StorageError> {
    fn from(err: StorageError) -> MerkleTreeError<StorageError> {
        MerkleTreeError::StorageError(err)
    }
}

// Obtained by creating a tree with a single leaf, removing that leaf, and then getting
// the tree node.
pub const EMPTY_ROOT: Bytes32 = [
    83, 80, 65, 82, 83, 69, 95, 77, 69, 82, 75, 76, 69, 95, 80, 76, 65, 67, 69, 72, 79,
    76, 68, 69, 82, 95, 72, 65, 83, 72, 95, 95,
];

#[derive(Debug, Clone)]
pub struct MerkleTreeStorage<
    NodeTableType,
    ValueTableType,
    // TODO: RightmostLeafTableType is used only when the in JellyFishMerkleTreeRestore,
    // which we do not use.
    // This should be removed, as currently the rightmost leaf is not updated correctly
    // when nodes are removed.
    RightmostLeafTableType,
    LatestRootVersionTableType,
    StorageType,
> {
    pub(crate) storage: alloc::sync::Arc<RwLock<StorageType>>,
    // Todo: remove as not needed
    leaves_count: alloc::sync::Arc<AtomicU64>,
    phantom_table: PhantomData<(
        NodeTableType,
        ValueTableType,
        RightmostLeafTableType,
        LatestRootVersionTableType,
    )>,
}

impl<
        NodeTableType,
        ValueTableType,
        RightmostLeafTableType,
        LatestRootVersionTableType,
        StorageType,
    > TreeWriter
    for MerkleTreeStorage<
        NodeTableType,
        ValueTableType,
        RightmostLeafTableType,
        LatestRootVersionTableType,
        StorageType,
    >
where
    NodeTableType: Mappable<
        Key = NodeKey,
        Value = jmt::storage::Node,
        OwnedValue = jmt::storage::Node,
    >,
    ValueTableType: Mappable<
        Key = jmt::KeyHash,
        Value = (jmt::Version, jmt::OwnedValue),
        OwnedValue = (jmt::Version, jmt::OwnedValue),
    >,
    RightmostLeafTableType: Mappable<
        Key = (),
        Value = (jmt::KeyHash, jmt::storage::NodeKey),
        OwnedValue = (jmt::KeyHash, jmt::storage::NodeKey),
    >,
    LatestRootVersionTableType: Mappable<Key = (), Value = u64, OwnedValue = u64>,
    StorageType: StorageMutate<NodeTableType>
        + StorageMutate<ValueTableType>
        + StorageMutate<RightmostLeafTableType>
        + StorageMutate<LatestRootVersionTableType>,
{
    fn write_node_batch(
        &self,
        node_batch: &jmt::storage::NodeBatch,
    ) -> anyhow::Result<()> {
        for (key, node) in node_batch.nodes() {
            let mut storage = self.storage
                // TODO: We need to check that mutable access to the storage is exclusive
                // If not, RefCell<Storage> will need to be replaced with RwLock<Storage>
                .write();
            StorageMutate::<NodeTableType>::insert(&mut *storage, key, node)
                .map_err(|_err| anyhow::anyhow!("Node table write Storage Error"))?;
            if key.nibble_path().is_empty() {
                // If the nibble path is empty, we are updating the root node.
                // We must also update the latest root version
                let newer_version =
                    StorageInspect::<LatestRootVersionTableType>::get(&*storage, &())
                        .map_err(|_e| {
                            anyhow::anyhow!("Latest root version read storage error")
                        })?
                        .map(|v| *v)
                        .filter(|v| *v >= key.version());
                // To check: it should never be the case that this check fails
                if newer_version.is_none() {
                    StorageMutate::<LatestRootVersionTableType>::insert(
                        &mut *storage,
                        &(),
                        &key.version(),
                    )
                    .map_err(|_e| {
                        anyhow::anyhow!("Latest root version write storage error")
                    })?;
                }
            }

            // need to update the rightmost leaf
            match node {
                jmt::storage::Node::Leaf(leaf) => {
                    // update the preimage table
                    let rightmost_leaf =
                        StorageInspect::<RightmostLeafTableType>::get(&*storage, &())
                            .map_err(|_e| {
                                anyhow::anyhow!("Rightmost leaf read storage error")
                            })?;

                    if let Some(key_with_node) = rightmost_leaf {
                        if leaf.key_hash() >= key_with_node.0 {
                            StorageMutate::<RightmostLeafTableType>::insert(
                                &mut *storage,
                                &(),
                                &(leaf.key_hash(), key.clone()),
                            )
                            .map_err(|_e| {
                                anyhow::anyhow!("Rightmost leaf write storage error")
                            })?;
                        }
                    } else {
                        StorageMutate::<RightmostLeafTableType>::insert(
                            &mut *storage,
                            &(),
                            &(leaf.key_hash(), key.clone()),
                        )
                        .map_err(|_e| {
                            anyhow::anyhow!("Rightmost leaf write storage error")
                        })?;
                    }
                }
                _ => {}
            };

            for ((version, key_hash), value) in node_batch.values() {
                match value {
                    None => {
                        let old = StorageMutate::<ValueTableType>::take(
                            &mut *storage,
                            key_hash,
                        )
                        .map_err(|_e| anyhow::anyhow!("Version Storage Error"))?;
                        if old.is_some() {
                            self.leaves_count
                                .fetch_sub(1, core::sync::atomic::Ordering::Release);
                        }
                    }
                    Some(value) => {
                        let old = StorageMutate::<ValueTableType>::replace(
                            &mut *storage,
                            key_hash,
                            &(*version, value.clone()),
                        )
                        .map_err(|_e| anyhow::anyhow!("Version Storage Error"))?;
                        if old.is_none() {
                            self.leaves_count
                                .fetch_add(1, core::sync::atomic::Ordering::Release);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl<
        NodeTableType,
        ValueTableType,
        RightmostLeafTableType,
        LatestRootVersionTableType,
        StorageType,
    > TreeReader
    for MerkleTreeStorage<
        NodeTableType,
        ValueTableType,
        RightmostLeafTableType,
        LatestRootVersionTableType,
        StorageType,
    >
where
    NodeTableType: Mappable<
        Key = NodeKey,
        Value = jmt::storage::Node,
        OwnedValue = jmt::storage::Node,
    >,
    ValueTableType: Mappable<
        Key = jmt::KeyHash,
        Value = (jmt::Version, jmt::OwnedValue),
        OwnedValue = (jmt::Version, jmt::OwnedValue),
    >,
    RightmostLeafTableType: Mappable<
        Key = (),
        Value = (jmt::KeyHash, jmt::storage::NodeKey),
        OwnedValue = (jmt::KeyHash, jmt::storage::NodeKey),
    >,
    StorageType: StorageInspect<NodeTableType>
        + StorageInspect<ValueTableType>
        + StorageInspect<RightmostLeafTableType>,
{
    fn get_node_option(
        &self,
        node_key: &NodeKey,
    ) -> anyhow::Result<Option<jmt::storage::Node>> {
        let storage = self.storage.read();
        let get_result = StorageInspect::<NodeTableType>::get(&*storage, node_key)
            .map_err(|_e| anyhow::anyhow!("Storage Error"))?;
        let node = get_result.map(|node| node.into_owned());

        Ok(node)
    }

    fn get_value_option(
        &self,
        max_version: jmt::Version,
        key_hash: jmt::KeyHash,
    ) -> anyhow::Result<Option<jmt::OwnedValue>> {
        let storage = self.storage.read();
        let Some(value) = StorageInspect::<ValueTableType>::get(&*storage, &key_hash)
            .map_err(|_e| anyhow::anyhow!("Version Storage Error"))?
            .filter(|v| v.0 <= max_version)
            .map(|v| v.into_owned().1)
        else {
            return Ok(None)
        };
        // Retrieve current version of key

        return Ok(Some(value))
    }

    fn get_rightmost_leaf(
        &self,
    ) -> anyhow::Result<Option<(NodeKey, jmt::storage::LeafNode)>> {
        let storage = self.storage.read();
        let Some((_key_hash, node_key)) =
            StorageInspect::<RightmostLeafTableType>::get(&*storage, &())
                .map_err(|_e| anyhow::anyhow!("Rightmost leaf storage error"))?
                .map(|v| v.into_owned())
        else {
            return Ok(None)
        };

        let leaf = StorageInspect::<NodeTableType>::get(&*storage, &node_key)
            .map_err(|e| anyhow::anyhow!("Node storage error"))?
            .map(|v| v.into_owned());

        match leaf {
            Some(jmt::storage::Node::Leaf(leaf)) => Ok(Some((node_key, leaf))),
            _ => Err(anyhow::anyhow!(
                "Consistency error: node stored for rightmost leaf is not a leaf node"
            )),
        }
    }
}

impl<
        NodeTableType,
        ValueTableType,
        RightmostLeafTableType,
        LatestRootVersionTableType,
        StorageType,
    > HasPreimage
    for MerkleTreeStorage<
        NodeTableType,
        ValueTableType,
        RightmostLeafTableType,
        LatestRootVersionTableType,
        StorageType,
    >
where
    ValueTableType: Mappable<
        Key = jmt::KeyHash,
        Value = (jmt::Version, jmt::OwnedValue),
        OwnedValue = (jmt::Version, jmt::OwnedValue),
    >,
    StorageType: StorageInspect<ValueTableType>,
{
    fn preimage(&self, key_hash: jmt::KeyHash) -> anyhow::Result<Option<Vec<u8>>> {
        let storage = self.storage.read();
        let preimage = StorageInspect::<ValueTableType>::get(&*storage, &key_hash)
            .map_err(|_e| anyhow::anyhow!("Preimage storage error"))?
            .map(|v| v.into_owned().1);

        Ok(preimage)
    }
}

impl<
        NodeTableType,
        ValueTableType,
        RightmostLeafTableType,
        LatestRootVersionTableType,
        StorageType,
    >
    MerkleTreeStorage<
        NodeTableType,
        ValueTableType,
        RightmostLeafTableType,
        LatestRootVersionTableType,
        StorageType,
    >
where
    NodeTableType: Mappable<
        Key = NodeKey,
        Value = jmt::storage::Node,
        OwnedValue = jmt::storage::Node,
    >,
    ValueTableType: Mappable<
        Key = jmt::KeyHash,
        Value = (jmt::Version, jmt::OwnedValue),
        OwnedValue = (jmt::Version, jmt::OwnedValue),
    >,
    RightmostLeafTableType: Mappable<
        Key = (),
        Value = (jmt::KeyHash, jmt::storage::NodeKey),
        OwnedValue = (jmt::KeyHash, jmt::storage::NodeKey),
    >,
    LatestRootVersionTableType: Mappable<Key = (), Value = u64, OwnedValue = u64>,
    StorageType: StorageMutate<NodeTableType>
        + StorageMutate<ValueTableType>
        + StorageMutate<RightmostLeafTableType>
        + StorageMutate<LatestRootVersionTableType>,
{
    fn get_latest_root_version(&self) -> anyhow::Result<Option<u64>> {
        let storage = self.storage.read();
        let version = StorageInspect::<LatestRootVersionTableType>::get(&*storage, &())
            .map_err(|_e| anyhow::anyhow!("Latest root version storage error"))?
            .map(|v| *v);

        Ok(version)
    }

    fn as_jmt<'a>(&'a self) -> Sha256Jmt<'a, Self> {
        JellyfishMerkleTree::new(&self)
    }

    pub const fn empty_root() -> &'static Bytes32 {
        &EMPTY_ROOT
    }

    pub fn storage_read(&self) -> spin::RwLockReadGuard<StorageType> {
        self.storage.read()
    }

    pub fn storage_write(&self) -> spin::RwLockWriteGuard<StorageType> {
        self.storage.write()
    }

    // TODO: What to do with errors?
    pub fn root(&self) -> Bytes32 {
        // We need to know the version of the root node.

        let Some(version) = self.get_latest_root_version().unwrap_or_default() else {
            return *Self::empty_root();
        };

        self.as_jmt()
            .get_root_hash(version)
            .map(|root_hash| root_hash.0)
            .unwrap_or_default()
    }

    pub fn load(storage: StorageType, root: &Bytes32) -> Result<Self, anyhow::Error> {
        let merkle_tree = Self::new(storage);
        let root_from_storage = merkle_tree.root();
        //
        if *root == root_from_storage {
            Ok(merkle_tree)
        } else {
            Err(anyhow::anyhow!("Root hash mismatch"))
        }
    }

    pub fn leaves_count(&self) -> u64 {
        self.leaves_count
            .load(core::sync::atomic::Ordering::Acquire)
    }

    pub fn new(storage: StorageType) -> Self {
        let storage = Arc::new(RwLock::new(storage));
        Self {
            storage,
            // TODO: Remove this, as it is not accurate and not needed
            leaves_count: Arc::new(AtomicU64::new(0)),
            phantom_table: Default::default(),
        }
    }

    pub fn from_set<B, I, D>(storage: StorageType, set: I) -> anyhow::Result<Self>
    where
        I: Iterator<Item = (B, D)>,
        B: Into<Bytes32>,
        D: AsRef<[u8]>,
    {
        let tree = Self::new(storage);
        let jmt = tree.as_jmt();
        // We assume that we are constructing a new Merkle Tree, hence the version is set
        // at 0
        // value_set: impl IntoIterator<Item = (KeyHash, Option<OwnedValue>)>,

        let version = 0;
        let update_batch = set.map(|(key, data)| {
            let key_hash = jmt::KeyHash(key.into());
            // Sad, but jmt requires an owned value
            let value = data.as_ref().to_vec();
            (key_hash, Some(value))
        });
        // This writes the values into the tree cache. This function returns the tree
        // updates that must be written into storage
        let (_root_hash, updates) = jmt
            .put_value_set(update_batch, version)
            .map_err(|e| anyhow::anyhow!("Error updating tree: {:?}", e))?;
        // TODO: Should we check the stale node indexes as well?
        let node_updates = updates.node_batch;
        <Self as TreeWriter>::write_node_batch(&tree, &node_updates)?;

        Ok(tree)
    }

    pub fn update(
        &mut self,
        key: MerkleTreeKey,
        data: &[u8],
    ) -> Result<(), anyhow::Error> {
        let key_hash = jmt::KeyHash(*key);
        // If data.is_empty() we remove the value from the jmt
        let value = if data.is_empty() {
            None
        } else {
            Some(data.to_vec())
        };
        // TODO : We could update version once per block, but here we
        // update version for each update operation.
        let version = self
            .get_latest_root_version()
            .unwrap_or_default()
            .unwrap_or_default()
            .saturating_add(1);
        let update_batch = [(key_hash, value); 1];
        let (_root_hash, updates) = self.as_jmt().put_value_set(update_batch, version)?;
        // TODO: Figure out what to do with stale node indexes
        let node_updates = updates.node_batch;
        <Self as TreeWriter>::write_node_batch(&self, &node_updates);
        return Ok(())
    }

    pub fn delete(&mut self, key: MerkleTreeKey) -> Result<(), anyhow::Error> {
        self.update(key, &[])
    }
}

// #[cfg(test)]
// mod test {
// use super::MerkleTreeError;
// use crate::{
// binary::{
// empty_sum,
// leaf_sum,
// node_sum,
// Node,
// Primitive,
// },
// common::StorageMap,
// };
// use fuel_merkle_test_helpers::TEST_DATA;
// use fuel_storage::{
// Mappable,
// StorageInspect,
// StorageMutate,
// };
//
// use alloc::vec::Vec;
//
// #[derive(Debug)]
// struct TestTable;
//
// impl Mappable for TestTable {
// type Key = Self::OwnedKey;
// type OwnedKey = u64;
// type OwnedValue = Primitive;
// type Value = Self::OwnedValue;
// }
//
// #[test]
// fn test_push_builds_internal_tree_structure() {
// let mut storage_map = StorageMap::<TestTable>::new();
// let mut tree = MerkleTree::new(&mut storage_map);
//
// let data = &TEST_DATA[0..7]; // 7 leaves
// for datum in data.iter() {
// let _ = tree.push(datum);
// }
//
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
//
// let leaf_0 = leaf_sum(data[0]);
// let leaf_1 = leaf_sum(data[1]);
// let leaf_2 = leaf_sum(data[2]);
// let leaf_3 = leaf_sum(data[3]);
// let leaf_4 = leaf_sum(data[4]);
// let leaf_5 = leaf_sum(data[5]);
// let leaf_6 = leaf_sum(data[6]);
// let node_1 = node_sum(&leaf_0, &leaf_1);
// let node_5 = node_sum(&leaf_2, &leaf_3);
// let node_3 = node_sum(&node_1, &node_5);
// let node_9 = node_sum(&leaf_4, &leaf_5);
//
// let s_leaf_0 = storage_map.get(&0).unwrap().unwrap();
// let s_leaf_1 = storage_map.get(&2).unwrap().unwrap();
// let s_leaf_2 = storage_map.get(&4).unwrap().unwrap();
// let s_leaf_3 = storage_map.get(&6).unwrap().unwrap();
// let s_leaf_4 = storage_map.get(&8).unwrap().unwrap();
// let s_leaf_5 = storage_map.get(&10).unwrap().unwrap();
// let s_leaf_6 = storage_map.get(&12).unwrap().unwrap();
// let s_node_1 = storage_map.get(&1).unwrap().unwrap();
// let s_node_5 = storage_map.get(&5).unwrap().unwrap();
// let s_node_9 = storage_map.get(&9).unwrap().unwrap();
// let s_node_3 = storage_map.get(&3).unwrap().unwrap();
//
// assert_eq!(*Node::from(s_leaf_0.into_owned()).hash(), leaf_0);
// assert_eq!(*Node::from(s_leaf_1.into_owned()).hash(), leaf_1);
// assert_eq!(*Node::from(s_leaf_2.into_owned()).hash(), leaf_2);
// assert_eq!(*Node::from(s_leaf_3.into_owned()).hash(), leaf_3);
// assert_eq!(*Node::from(s_leaf_4.into_owned()).hash(), leaf_4);
// assert_eq!(*Node::from(s_leaf_5.into_owned()).hash(), leaf_5);
// assert_eq!(*Node::from(s_leaf_6.into_owned()).hash(), leaf_6);
// assert_eq!(*Node::from(s_node_1.into_owned()).hash(), node_1);
// assert_eq!(*Node::from(s_node_5.into_owned()).hash(), node_5);
// assert_eq!(*Node::from(s_node_9.into_owned()).hash(), node_9);
// assert_eq!(*Node::from(s_node_3.into_owned()).hash(), node_3);
// }
//
// #[test]
// fn load_returns_a_valid_tree() {
// const LEAVES_COUNT: u64 = 2u64.pow(16) - 1;
//
// let mut storage_map = StorageMap::<TestTable>::new();
//
// let expected_root = {
// let mut tree = MerkleTree::new(&mut storage_map);
// let data = (0u64..LEAVES_COUNT)
// .map(|i| i.to_be_bytes())
// .collect::<Vec<_>>();
// for datum in data.iter() {
// let _ = tree.push(datum);
// }
// tree.root()
// };
//
// let root = {
// let tree = MerkleTree::load(&mut storage_map, LEAVES_COUNT).unwrap();
// tree.root()
// };
//
// assert_eq!(expected_root, root);
// }
//
// #[test]
// fn load_returns_empty_tree_for_0_leaves() {
// const LEAVES_COUNT: u64 = 0;
//
// let expected_root = *MerkleTree::<(), ()>::empty_root();
//
// let root = {
// let mut storage_map = StorageMap::<TestTable>::new();
// let tree = MerkleTree::load(&mut storage_map, LEAVES_COUNT).unwrap();
// tree.root()
// };
//
// assert_eq!(expected_root, root);
// }
//
// #[test]
// fn load_returns_a_load_error_if_the_storage_is_not_valid_for_the_leaves_count() {
// const LEAVES_COUNT: u64 = 5;
//
// let mut storage_map = StorageMap::<TestTable>::new();
//
// let mut tree = MerkleTree::new(&mut storage_map);
// let data = (0u64..LEAVES_COUNT)
// .map(|i| i.to_be_bytes())
// .collect::<Vec<_>>();
// for datum in data.iter() {
// let _ = tree.push(datum);
// }
//
// let err = MerkleTree::load(&mut storage_map, LEAVES_COUNT * 2)
// .expect_err("Expected load() to return Error; got Ok");
// assert!(matches!(err, MerkleTreeError::LoadError(_)));
// }
//
// #[test]
// fn root_returns_the_empty_root_for_0_leaves() {
// let mut storage_map = StorageMap::<TestTable>::new();
// let tree = MerkleTree::new(&mut storage_map);
//
// let root = tree.root();
// assert_eq!(root, empty_sum().clone());
// }
//
// #[test]
// fn root_returns_the_merkle_root_for_1_leaf() {
// let mut storage_map = StorageMap::<TestTable>::new();
// let mut tree = MerkleTree::new(&mut storage_map);
//
// let data = &TEST_DATA[0..1]; // 1 leaf
// for datum in data.iter() {
// let _ = tree.push(datum);
// }
//
// let leaf_0 = leaf_sum(data[0]);
//
// let root = tree.root();
// assert_eq!(root, leaf_0);
// }
//
// #[test]
// fn root_returns_the_merkle_root_for_7_leaves() {
// let mut storage_map = StorageMap::<TestTable>::new();
// let mut tree = MerkleTree::new(&mut storage_map);
//
// let data = &TEST_DATA[0..7]; // 7 leaves
// for datum in data.iter() {
// let _ = tree.push(datum);
// }
//
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
//
// let leaf_0 = leaf_sum(data[0]);
// let leaf_1 = leaf_sum(data[1]);
// let leaf_2 = leaf_sum(data[2]);
// let leaf_3 = leaf_sum(data[3]);
// let leaf_4 = leaf_sum(data[4]);
// let leaf_5 = leaf_sum(data[5]);
// let leaf_6 = leaf_sum(data[6]);
//
// let node_1 = node_sum(&leaf_0, &leaf_1);
// let node_5 = node_sum(&leaf_2, &leaf_3);
// let node_3 = node_sum(&node_1, &node_5);
// let node_9 = node_sum(&leaf_4, &leaf_5);
// let node_11 = node_sum(&node_9, &leaf_6);
// let node_7 = node_sum(&node_3, &node_11);
//
// let root = tree.root();
// assert_eq!(root, node_7);
// }
//
// #[test]
// fn prove_returns_invalid_proof_index_error_for_0_leaves() {
// let mut storage_map = StorageMap::<TestTable>::new();
// let tree = MerkleTree::new(&mut storage_map);
//
// let err = tree
// .prove(0)
// .expect_err("Expected prove() to return Error; got Ok");
// assert!(matches!(err, MerkleTreeError::InvalidProofIndex(0)));
// }
//
// #[test]
// fn prove_returns_invalid_proof_index_error_when_index_is_greater_than_number_of_leaves(
// ) {
// let mut storage_map = StorageMap::<TestTable>::new();
// let mut tree = MerkleTree::new(&mut storage_map);
//
// let data = &TEST_DATA[0..5]; // 5 leaves
// for datum in data.iter() {
// let _ = tree.push(datum);
// }
//
// let err = tree
// .prove(10)
// .expect_err("Expected prove() to return Error; got Ok");
// assert!(matches!(err, MerkleTreeError::InvalidProofIndex(10)))
// }
//
// #[test]
// fn prove_returns_the_merkle_root_and_proof_set_for_1_leaf() {
// let mut storage_map = StorageMap::<TestTable>::new();
// let mut tree = MerkleTree::new(&mut storage_map);
//
// let data = &TEST_DATA[0..1]; // 1 leaf
// for datum in data.iter() {
// let _ = tree.push(datum);
// }
//
// let leaf_0 = leaf_sum(data[0]);
//
// {
// let (root, proof_set) = tree.prove(0).unwrap();
// assert_eq!(root, leaf_0);
// assert!(proof_set.is_empty());
// }
// }
//
// #[test]
// fn prove_returns_the_merkle_root_and_proof_set_for_4_leaves() {
// let mut storage_map = StorageMap::<TestTable>::new();
// let mut tree = MerkleTree::new(&mut storage_map);
//
// let data = &TEST_DATA[0..4]; // 4 leaves
// for datum in data.iter() {
// let _ = tree.push(datum);
// }
//
//       03
//      /  \
//     /    \
//   01      05
//  /  \    /  \
// 00  02  04  06
// 00  01  02  03
//
// let leaf_0 = leaf_sum(data[0]);
// let leaf_1 = leaf_sum(data[1]);
// let leaf_2 = leaf_sum(data[2]);
// let leaf_3 = leaf_sum(data[3]);
//
// let node_1 = node_sum(&leaf_0, &leaf_1);
// let node_5 = node_sum(&leaf_2, &leaf_3);
// let node_3 = node_sum(&node_1, &node_5);
//
// {
// let (root, proof_set) = tree.prove(0).unwrap();
// assert_eq!(root, node_3);
// assert_eq!(proof_set[0], leaf_1);
// assert_eq!(proof_set[1], node_5);
// }
// {
// let (root, proof_set) = tree.prove(1).unwrap();
// assert_eq!(root, node_3);
// assert_eq!(proof_set[0], leaf_0);
// assert_eq!(proof_set[1], node_5);
// }
// {
// let (root, proof_set) = tree.prove(2).unwrap();
// assert_eq!(root, node_3);
// assert_eq!(proof_set[0], leaf_3);
// assert_eq!(proof_set[1], node_1);
// }
// {
// let (root, proof_set) = tree.prove(3).unwrap();
// assert_eq!(root, node_3);
// assert_eq!(proof_set[0], leaf_2);
// assert_eq!(proof_set[1], node_1);
// }
// }
//
// #[test]
// fn prove_returns_the_merkle_root_and_proof_set_for_5_leaves() {
// let mut storage_map = StorageMap::<TestTable>::new();
// let mut tree = MerkleTree::new(&mut storage_map);
//
// let data = &TEST_DATA[0..5]; // 5 leaves
// for datum in data.iter() {
// let _ = tree.push(datum);
// }
//
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
//
// let leaf_0 = leaf_sum(data[0]);
// let leaf_1 = leaf_sum(data[1]);
// let leaf_2 = leaf_sum(data[2]);
// let leaf_3 = leaf_sum(data[3]);
// let leaf_4 = leaf_sum(data[4]);
//
// let node_1 = node_sum(&leaf_0, &leaf_1);
// let node_5 = node_sum(&leaf_2, &leaf_3);
// let node_3 = node_sum(&node_1, &node_5);
// let node_7 = node_sum(&node_3, &leaf_4);
//
// {
// let (root, proof_set) = tree.prove(0).unwrap();
// assert_eq!(root, node_7);
// assert_eq!(proof_set[0], leaf_1);
// assert_eq!(proof_set[1], node_5);
// assert_eq!(proof_set[2], leaf_4);
// }
// {
// let (root, proof_set) = tree.prove(1).unwrap();
// assert_eq!(root, node_7);
// assert_eq!(proof_set[0], leaf_0);
// assert_eq!(proof_set[1], node_5);
// assert_eq!(proof_set[2], leaf_4);
// }
// {
// let (root, proof_set) = tree.prove(2).unwrap();
// assert_eq!(root, node_7);
// assert_eq!(proof_set[0], leaf_3);
// assert_eq!(proof_set[1], node_1);
// assert_eq!(proof_set[2], leaf_4);
// }
// {
// let (root, proof_set) = tree.prove(3).unwrap();
// assert_eq!(root, node_7);
// assert_eq!(proof_set[0], leaf_2);
// assert_eq!(proof_set[1], node_1);
// assert_eq!(proof_set[2], leaf_4);
// }
// {
// let (root, proof_set) = tree.prove(4).unwrap();
// assert_eq!(root, node_7);
// assert_eq!(proof_set[0], node_3);
// }
// }
//
// #[test]
// fn prove_returns_the_merkle_root_and_proof_set_for_7_leaves() {
// let mut storage_map = StorageMap::<TestTable>::new();
// let mut tree = MerkleTree::new(&mut storage_map);
//
// let data = &TEST_DATA[0..7]; // 7 leaves
// for datum in data.iter() {
// let _ = tree.push(datum);
// }
//
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
//
// let leaf_0 = leaf_sum(data[0]);
// let leaf_1 = leaf_sum(data[1]);
// let leaf_2 = leaf_sum(data[2]);
// let leaf_3 = leaf_sum(data[3]);
// let leaf_4 = leaf_sum(data[4]);
// let leaf_5 = leaf_sum(data[5]);
// let leaf_6 = leaf_sum(data[6]);
//
// let node_1 = node_sum(&leaf_0, &leaf_1);
// let node_5 = node_sum(&leaf_2, &leaf_3);
// let node_3 = node_sum(&node_1, &node_5);
// let node_9 = node_sum(&leaf_4, &leaf_5);
// let node_11 = node_sum(&node_9, &leaf_6);
// let node_7 = node_sum(&node_3, &node_11);
//
// {
// let (root, proof_set) = tree.prove(0).unwrap();
// assert_eq!(root, node_7);
// assert_eq!(proof_set[0], leaf_1);
// assert_eq!(proof_set[1], node_5);
// assert_eq!(proof_set[2], node_11);
// }
// {
// let (root, proof_set) = tree.prove(1).unwrap();
// assert_eq!(root, node_7);
// assert_eq!(proof_set[0], leaf_0);
// assert_eq!(proof_set[1], node_5);
// assert_eq!(proof_set[2], node_11);
// }
// {
// let (root, proof_set) = tree.prove(2).unwrap();
// assert_eq!(root, node_7);
// assert_eq!(proof_set[0], leaf_3);
// assert_eq!(proof_set[1], node_1);
// assert_eq!(proof_set[2], node_11);
// }
// {
// let (root, proof_set) = tree.prove(3).unwrap();
// assert_eq!(root, node_7);
// assert_eq!(proof_set[0], leaf_2);
// assert_eq!(proof_set[1], node_1);
// assert_eq!(proof_set[2], node_11);
// }
// {
// let (root, proof_set) = tree.prove(4).unwrap();
// assert_eq!(root, node_7);
// assert_eq!(proof_set[0], leaf_5);
// assert_eq!(proof_set[1], leaf_6);
// assert_eq!(proof_set[2], node_3);
// }
// {
// let (root, proof_set) = tree.prove(5).unwrap();
// assert_eq!(root, node_7);
// assert_eq!(proof_set[0], leaf_4);
// assert_eq!(proof_set[1], leaf_6);
// assert_eq!(proof_set[2], node_3);
// }
// {
// let (root, proof_set) = tree.prove(6).unwrap();
// assert_eq!(root, node_7);
// assert_eq!(proof_set[0], node_9);
// assert_eq!(proof_set[1], node_3);
// }
// }
//
// #[test]
// fn reset_reverts_tree_to_empty_state() {
// let mut storage_map = StorageMap::<TestTable>::new();
// let mut tree = MerkleTree::new(&mut storage_map);
//
// let data = &TEST_DATA[0..4]; // 4 leaves
// for datum in data.iter() {
// let _ = tree.push(datum);
// }
//
// tree.reset();
//
// let root = tree.root();
// let expected_root = *MerkleTree::<(), ()>::empty_root();
// assert_eq!(root, expected_root);
//
// let data = &TEST_DATA[0..4]; // 4 leaves
// for datum in data.iter() {
// let _ = tree.push(datum);
// }
//
// let leaf_0 = leaf_sum(data[0]);
// let leaf_1 = leaf_sum(data[1]);
// let leaf_2 = leaf_sum(data[2]);
// let leaf_3 = leaf_sum(data[3]);
//
// let node_1 = node_sum(&leaf_0, &leaf_1);
// let node_5 = node_sum(&leaf_2, &leaf_3);
// let node_3 = node_sum(&node_1, &node_5);
//
// let root = tree.root();
// let expected_root = node_3;
// assert_eq!(root, expected_root);
// }
//
// #[test]
// fn load_overflows() {
// Given
// let storage_map = StorageMap::<TestTable>::new();
// const LEAVES_COUNT: u64 = u64::MAX;
//
// When
// let result = MerkleTree::load(storage_map, LEAVES_COUNT).map(|_| ());
//
// Then
// assert_eq!(result, Err(MerkleTreeError::TooLarge));
// }
//
// #[test]
// fn push_overflows() {
// Given
// let mut storage_map = StorageMap::<TestTable>::new();
// const LEAVES_COUNT: u64 = u64::MAX / 2;
// loop {
// let result = MerkleTree::load(&mut storage_map, LEAVES_COUNT).map(|_| ());
//
// if let Err(MerkleTreeError::LoadError(index)) = result {
// storage_map.insert(&index, &Primitive::default()).unwrap();
// } else {
// break;
// }
// }
//
// When
// let mut tree = MerkleTree::load(storage_map, LEAVES_COUNT)
// .expect("Expected `load()` to succeed");
// let _ = tree.push(&[]);
// let result = tree.push(&[]);
//
// Then
// assert_eq!(result, Err(MerkleTreeError::TooLarge));
// }
// }

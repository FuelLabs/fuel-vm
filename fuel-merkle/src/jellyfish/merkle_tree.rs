use crate::{
    common::Bytes32,
    sparse::MerkleTreeKey,
    storage::{
        Mappable,
        StorageInspect,
        StorageMutate,
    },
};

use alloc::sync::Arc;
use core::marker::PhantomData;
use spin::{
    rwlock::RwLock,
    RwLockReadGuard,
    RwLockWriteGuard,
};

use jmt::{
    storage::{
        Node as JmtNode,
        NodeKey as JmtNodeKey,
        TreeWriter,
    },
    JellyfishMerkleTree,
    Sha256Jmt,
};

use crate::jellyfish::proof::{
    ExclusionProof,
    InclusionProof,
    MerkleProof,
};

#[derive(Debug, Clone, derive_more::Display, PartialEq, Eq)]
pub enum MerkleTreeError<StorageError> {
    #[display(fmt = "{}", _0)]
    StorageError(StorageError),
}

impl<StorageError> From<StorageError> for MerkleTreeError<StorageError> {
    fn from(err: StorageError) -> MerkleTreeError<StorageError> {
        MerkleTreeError::StorageError(err)
    }
}

// Obtained by creating an empty tree.
pub const EMPTY_ROOT: Bytes32 = [
    83, 80, 65, 82, 83, 69, 95, 77, 69, 82, 75, 76, 69, 95, 80, 76, 65, 67, 69, 72, 79,
    76, 68, 69, 82, 95, 72, 65, 83, 72, 95, 95,
];

#[derive(Debug, Clone)]
pub struct JellyfishMerkleTreeStorage<
    NodeTableType,
    ValueTableType,
    LatestRootVersionTableType,
    StorageType,
> {
    inner: Arc<RwLock<StorageType>>,
    phantom_table:
        PhantomData<(NodeTableType, ValueTableType, LatestRootVersionTableType)>,
}

impl<NodeTableType, ValueTableType, LatestRootVersionTableType, StorageType>
    JellyfishMerkleTreeStorage<
        NodeTableType,
        ValueTableType,
        LatestRootVersionTableType,
        StorageType,
    >
{
    pub const fn empty_root() -> &'static Bytes32 {
        &EMPTY_ROOT
    }

    pub fn storage_read(&self) -> RwLockReadGuard<StorageType> {
        self.inner.read()
    }

    pub fn storage_write(&self) -> RwLockWriteGuard<StorageType> {
        self.inner.write()
    }
}

impl<NodeTableType, ValueTableType, LatestRootVersionTableType, StorageType>
    JellyfishMerkleTreeStorage<
        NodeTableType,
        ValueTableType,
        LatestRootVersionTableType,
        StorageType,
    >
where
    LatestRootVersionTableType: Mappable<Key = (), Value = u64, OwnedValue = u64>,
    StorageType: StorageInspect<LatestRootVersionTableType>,
{
    fn get_latest_root_version(&self) -> anyhow::Result<Option<u64>> {
        let storage = self.storage_read();
        let version = <StorageType as StorageInspect<LatestRootVersionTableType>>::get(
            &*storage,
            &(),
        )
        .map_err(|_e| anyhow::anyhow!("Latest root version storage error"))?
        .map(|v| *v);

        Ok(version)
    }
}

impl<NodeTableType, ValueTableType, LatestRootVersionTableType, StorageType>
    JellyfishMerkleTreeStorage<
        NodeTableType,
        ValueTableType,
        LatestRootVersionTableType,
        StorageType,
    >
where
    NodeTableType: Mappable<Key = JmtNodeKey, Value = JmtNode, OwnedValue = JmtNode>,
    ValueTableType: Mappable<
        Key = jmt::KeyHash,
        Value = (jmt::Version, jmt::OwnedValue),
        OwnedValue = (jmt::Version, jmt::OwnedValue),
    >,
    StorageType: StorageInspect<NodeTableType> + StorageInspect<ValueTableType>,
{
    // Requires TreeReader + HasPreimage
    // TreeReader requires StorageInspect<NodeTableType> and
    // StorageInspect<ValueTableType> HasPreimage requires
    // StorageInspect<ValueTableType>
    fn as_jmt<'a>(&'a self) -> Sha256Jmt<'a, Self> {
        JellyfishMerkleTree::new(&self)
    }
}

impl<NodeTableType, ValueTableType, LatestRootVersionTableType, StorageType>
    JellyfishMerkleTreeStorage<
        NodeTableType,
        ValueTableType,
        LatestRootVersionTableType,
        StorageType,
    >
where
    NodeTableType: Mappable<Key = JmtNodeKey, Value = JmtNode, OwnedValue = JmtNode>,
    ValueTableType: Mappable<
        Key = jmt::KeyHash,
        Value = (jmt::Version, jmt::OwnedValue),
        OwnedValue = (jmt::Version, jmt::OwnedValue),
    >,
    LatestRootVersionTableType: Mappable<Key = (), Value = u64, OwnedValue = u64>,
    StorageType: StorageInspect<NodeTableType>
        + StorageInspect<ValueTableType>
        + StorageInspect<LatestRootVersionTableType>,
{
    // TODO: What to do with errors?
    // get_latest_root_version() requires StorageInspect<LatestRootVersionTableType>
    // as_jmt() requires TreeReader, which requires StorageInspect<NodeTableType> and
    // StorageInspect<ValueTableType>. Therefore this function requires
    // StorageInspect<LatestRootVersionTableType>, StorageInspect<NodeTableType>, and
    // StorageInspect<ValueTableType>.
    pub fn root(&self) -> anyhow::Result<Bytes32> {
        // We need to know the version of the root node.
        let version = self
            .get_latest_root_version()?
            .ok_or(anyhow::anyhow!("Error getting latest root version"))?;

        self.as_jmt()
            .get_root_hash(version)
            .map(|root_hash| root_hash.0)
    }

    pub fn load(storage: StorageType, root: &Bytes32) -> Result<Self, anyhow::Error> {
        let inner = Arc::new(RwLock::new(storage));
        let merkle_tree = Self {
            inner,
            phantom_table: PhantomData,
        };
        // If the storage is not initialized, this function will fail.
        // TODO: This should be tested.
        let root_from_storage = merkle_tree.root()?;
        //
        if *root == root_from_storage {
            Ok(merkle_tree)
        } else {
            Err(anyhow::anyhow!("Root hash mismatch"))
        }
    }

    pub fn generate_proof(
        &self,
        key: &MerkleTreeKey,
    ) -> Result<MerkleProof, anyhow::Error> {
        let jmt = self.as_jmt();
        let key_hash = jmt::KeyHash(**key);
        let version = self
            .get_latest_root_version()
            .unwrap_or_default()
            .unwrap_or_default();
        let (value_vec, proof) = jmt.get_with_proof(key_hash, version)?;
        let proof = match value_vec {
            Some(value) => MerkleProof::Inclusion(InclusionProof {
                proof,
                key: key_hash,
                value,
            }),
            None => MerkleProof::Exclusion(ExclusionProof {
                proof,
                key: key_hash,
            }),
        };
        Ok(proof)
    }
}

impl<NodeTableType, ValueTableType, LatestRootVersionTableType, StorageType>
    JellyfishMerkleTreeStorage<
        NodeTableType,
        ValueTableType,
        LatestRootVersionTableType,
        StorageType,
    >
where
    NodeTableType: Mappable<Key = JmtNodeKey, Value = JmtNode, OwnedValue = JmtNode>,
    ValueTableType: Mappable<
        Key = jmt::KeyHash,
        Value = (jmt::Version, jmt::OwnedValue),
        OwnedValue = (jmt::Version, jmt::OwnedValue),
    >,
    LatestRootVersionTableType: Mappable<Key = (), Value = u64, OwnedValue = u64>,
    StorageType: StorageMutate<NodeTableType>
        + StorageMutate<ValueTableType>
        + StorageMutate<LatestRootVersionTableType>,
{
    // Because we insert and remove a node, we need to have StorageType:
    // StorageMutate<NodeTableType> + StorageMutate<ValueTableType> +
    // StorageMutate<LatestRootVersionTableType>
    pub fn new(storage: StorageType) -> anyhow::Result<Self> {
        let inner = Arc::new(RwLock::new(storage));
        let mut tree = Self {
            inner,
            phantom_table: PhantomData,
        };
        // Inclusion and Exclusion proof require that the root is set, hence we add it
        // here. Jmt does not make the constructor for `NibblePath` accessible, so
        // we add and remove a node instead.
        // TODO: Find a way to set the root without adding and deleting a node
        let mock_key = MerkleTreeKey::new(Bytes32::default());
        let mock_value = vec![0u8];
        tree.update(mock_key, &mock_value)?;
        tree.delete(mock_key)?;

        Ok(tree)
    }

    pub fn from_set<B, I, D>(storage: StorageType, set: I) -> anyhow::Result<Self>
    where
        I: Iterator<Item = (B, D)>,
        B: Into<Bytes32>,
        D: AsRef<[u8]>,
    {
        let tree = Self::new(storage)?;
        let jmt = tree.as_jmt();
        // We assume that we are constructing a new Merkle Tree.
        // We start from version 2 to be consistent with the version obtained
        // when returning a new tree.
        // TODO: Change Self::new so that the initial version of a new tree is 0.
        let version = 2;
        let update_batch = set.map(|(key, data)| {
            let key_hash = jmt::KeyHash(key.into());
            // Sad, but jmt requires an owned value
            // TODO: We should consider forking jmt to allow for borrowed values
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

    // TODO: We should have a corresponding function to batch udpates and increment the
    // version only once
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
        <Self as TreeWriter>::write_node_batch(&self, &node_updates)?;
        let stale_nodes = updates.stale_node_index_batch;
        let mut storage_write_guard = self.storage_write();
        for stale_node_index in stale_nodes {
            let node_key = stale_node_index.node_key;
            StorageMutate::<NodeTableType>::remove(&mut *storage_write_guard, &node_key)
                .map_err(|_e| anyhow::anyhow!("Error removing node"))?;
        }
        return Ok(())
    }

    pub fn delete(&mut self, key: MerkleTreeKey) -> Result<(), anyhow::Error> {
        self.update(key, &[])
    }
}

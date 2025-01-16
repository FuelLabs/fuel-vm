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
use core::marker::PhantomData;
use spin::rwlock::RwLock;

use jmt::{
    storage::{
        HasPreimage,
        LeafNode as JmtLeafNode,
        Node as JmtNode,
        NodeBatch as JmtNodeBatch,
        NodeKey as JmtNodeKey,
        TreeReader,
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
    inner: alloc::sync::Arc<RwLock<StorageType>>,
    phantom_table:
        PhantomData<(NodeTableType, ValueTableType, LatestRootVersionTableType)>,
}

impl<NodeTableType, ValueTableType, LatestRootVersionTableType, StorageType> TreeWriter
    for JellyfishMerkleTreeStorage<
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
    fn write_node_batch(&self, node_batch: &JmtNodeBatch) -> anyhow::Result<()> {
        for (key, node) in node_batch.nodes() {
            let mut storage = self.inner
                // TODO: We need to check that mutable access to the storage is exclusive
                // If not, RefCell<Storage> will need to be replaced with RwLock<Storage>
                .write();
            <StorageType as StorageMutate<NodeTableType>>::insert(
                &mut *storage,
                key,
                node,
            )
            .map_err(|_err| anyhow::anyhow!("Node table write Storage Error"))?;
            if key.nibble_path().is_empty() {
                // If the nibble path is empty, we are updating the root node.
                // We must also update the latest root version
                let newer_version = <StorageType as StorageInspect<
                    LatestRootVersionTableType,
                >>::get(&*storage, &())
                .map_err(|_e| anyhow::anyhow!("Latest root version read storage error"))?
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

            for ((version, key_hash), value) in node_batch.values() {
                match value {
                    None => {
                        let _old = <StorageType as StorageMutate<ValueTableType>>::take(
                            &mut *storage,
                            key_hash,
                        )
                        .map_err(|_e| anyhow::anyhow!("Version Storage Error"))?;
                    }
                    Some(value) => {
                        let _old =
                            <StorageType as StorageMutate<ValueTableType>>::replace(
                                &mut *storage,
                                key_hash,
                                &(*version, value.clone()),
                            )
                            .map_err(|_e| anyhow::anyhow!("Version Storage Error"))?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl<NodeTableType, ValueTableType, LatestRootVersionTableType, StorageType> TreeReader
    for JellyfishMerkleTreeStorage<
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
    fn get_node_option(&self, node_key: &JmtNodeKey) -> anyhow::Result<Option<JmtNode>> {
        let storage = self.inner.read();
        let get_result =
            <StorageType as StorageInspect<NodeTableType>>::get(&*storage, node_key)
                .map_err(|_e| anyhow::anyhow!("Storage Error"))?;
        let node = get_result.map(|node| node.into_owned());

        Ok(node)
    }

    fn get_value_option(
        &self,
        max_version: jmt::Version,
        key_hash: jmt::KeyHash,
    ) -> anyhow::Result<Option<jmt::OwnedValue>> {
        let storage = self.inner.read();
        let Some(value) =
            <StorageType as StorageInspect<ValueTableType>>::get(&*storage, &key_hash)
                .map_err(|_e| anyhow::anyhow!("Version Storage Error"))?
                .filter(|v| v.0 <= max_version)
                .map(|v| v.into_owned().1)
        else {
            return Ok(None)
        };
        // Retrieve current version of key

        return Ok(Some(value))
    }

    fn get_rightmost_leaf(&self) -> anyhow::Result<Option<(JmtNodeKey, JmtLeafNode)>> {
        unimplemented!(
            "Righmost leaf is used only when restoring the tree, which we do not support"
        )
    }
}

impl<NodeTableType, ValueTableType, LatestRootVersionTableType, StorageType> HasPreimage
    for JellyfishMerkleTreeStorage<
        NodeTableType,
        ValueTableType,
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
        let storage = self.inner.read();
        let preimage =
            <StorageType as StorageInspect<ValueTableType>>::get(&*storage, &key_hash)
                .map_err(|_e| anyhow::anyhow!("Preimage storage error"))?
                .map(|v| v.into_owned().1);

        Ok(preimage)
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

    fn as_jmt<'a>(&'a self) -> Sha256Jmt<'a, Self> {
        JellyfishMerkleTree::new(&self)
    }

    pub const fn empty_root() -> &'static Bytes32 {
        &EMPTY_ROOT
    }

    pub fn storage_read(&self) -> spin::RwLockReadGuard<StorageType> {
        self.inner.read()
    }

    pub fn storage_write(&self) -> spin::RwLockWriteGuard<StorageType> {
        self.inner.write()
    }

    // TODO: What to do with errors?
    pub fn root(&self) -> anyhow::Result<Bytes32> {
        // We need to know the version of the root node.
        let version = self
            .get_latest_root_version()?
            .ok_or(anyhow::anyhow!("Error getting latest root version"))?;

        self.as_jmt()
            .get_root_hash(version)
            .map(|root_hash| root_hash.0)
    }

    pub fn _load(storage: StorageType, root: &Bytes32) -> Result<Self, anyhow::Error> {
        // TODO: Refactor, as new will now add an empty root
        let merkle_tree = Self::new(storage)?;
        let root_from_storage = merkle_tree.root()?;
        //
        if *root == root_from_storage {
            Ok(merkle_tree)
        } else {
            Err(anyhow::anyhow!("Root hash mismatch"))
        }
    }

    pub fn new(storage: StorageType) -> anyhow::Result<Self> {
        let inner = Arc::new(RwLock::new(storage));
        let mut tree = Self {
            inner,
            // TODO: Remove this, as it is not accurate and not needed
            phantom_table: Default::default(),
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
        // We assume that we are constructing a new Merkle Tree, hence the version is set
        // at 0
        // value_set: impl IntoIterator<Item = (KeyHash, Option<OwnedValue>)>,

        let version = 0;
        let update_batch = set.map(|(key, data)| {
            // We are forced to hash again to be consistent with ics23 proofs, which are
            // the only exposed proofs that support non-existence in the jmt
            // crate
            let key_hash = jmt::KeyHash::with::<sha2::Sha256>(key.into());
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
        let key_hash = jmt::KeyHash::with::<sha2::Sha256>(*key);
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

    pub fn generate_proof(
        &self,
        key: &MerkleTreeKey,
    ) -> Result<MerkleProof, anyhow::Error> {
        let jmt = self.as_jmt();
        let key_hash = jmt::KeyHash::with::<sha2::Sha256>(**key);
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

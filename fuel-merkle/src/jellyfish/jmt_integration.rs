use crate::storage::{
    Mappable,
    StorageInspect,
    StorageMutate,
};

use alloc::vec::Vec;

use jmt::storage::{
    HasPreimage,
    LeafNode as JmtLeafNode,
    Node as JmtNode,
    NodeBatch as JmtNodeBatch,
    NodeKey as JmtNodeKey,
    TreeReader,
    TreeWriter,
};

use super::merkle_tree::JellyfishMerkleTreeStorage;

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
            // TODO: Do we really need locks here?
            let mut storage = self.storage_write();
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
        let storage = self.storage_read();
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
        let storage = self.storage_read();
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
        let storage = self.storage_read();
        let preimage =
            <StorageType as StorageInspect<ValueTableType>>::get(&*storage, &key_hash)
                .map_err(|_e| anyhow::anyhow!("Preimage storage error"))?
                .map(|v| v.into_owned().1);

        Ok(preimage)
    }
}

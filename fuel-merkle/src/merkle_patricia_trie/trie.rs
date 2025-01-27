use core::marker::PhantomData;

use alloy_trie::nodes::{
    BranchNode,
    RlpNode,
    TrieNode,
};

use alloy_primitives::B256;
use fuel_storage::{
    Mappable,
    StorageInspect,
    StorageMutate,
};
use nybbles::Nibbles;

use crate::common::Bytes32;

pub struct Trie<Storage, NodesTable> {
    storage: Storage,
    root: Option<RlpNode>,
    _phantom: PhantomData<NodesTable>,
}

impl<Storage, NodesTableType> Trie<Storage, NodesTableType> {
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
            root: None,
            _phantom: PhantomData,
        }
    }
}

impl<StorageType, NodesTableType> Trie<StorageType, NodesTableType>
where
    StorageType: StorageMutate<NodesTableType, Error = anyhow::Error>,
    NodesTableType: Mappable<Key = B256, Value = TrieNode, OwnedValue = TrieNode>,
{
    pub fn child_at_nibble(
        &self,
        node: BranchNode,
        nibble: u8,
    ) -> anyhow::Result<Option<TrieNode>> {
        if nibble > 0x0f {
            Err(anyhow::anyhow!("Invalid nibble: {}", nibble))?
        };

        let node = node
            .child_hash(nibble)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;

        let node = self
            .storage
            .get(&node)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;

        Ok(None)
    }

    pub fn lookup(&self, root: B256, path: Nibbles) -> anyhow::Result<Option<Vec<B256>>> {
        // Lookup a node and return the list of nodes traversed
        // These should be the only nodes that will need to be modified when
        // inserting a new node in the tree
        let root = self
            .storage
            .get(&root)
            .map_err(|e| anyhow::anyhow!("{:?}", e));

        Ok(Some(alloc::vec![B256::ZERO]))
    }
}

// It might be an overkill to keep track of the changes at this level,
// and apply them in sequence for each operation. But it makes the code cleaner

use alloy_trie::nodes::{
    RlpNode,
    TrieNode,
};
use fuel_storage::{
    Mappable,
    StorageMutate,
};

use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct Pending {
    to_delete: Vec<RlpNode>,
    to_insert: Vec<(RlpNode, TrieNode)>,
}

impl Pending {
    pub fn new() -> Self {
        Self {
            to_delete: Vec::new(),
            to_insert: Vec::new(),
        }
    }

    pub fn delete(&mut self, node: RlpNode) {
        self.to_delete.push(node);
    }

    pub fn insert(&mut self, node: RlpNode, value: TrieNode) {
        self.to_insert.push((node, value));
    }

    pub(crate) fn to_insert(&self) -> &Vec<(RlpNode, TrieNode)> {
        &self.to_insert
    }

    pub(crate) fn to_insert_mut(&mut self) -> &mut Vec<(RlpNode, TrieNode)> {
        &mut self.to_insert
    }

    pub(crate) fn to_delete(&self) -> &Vec<RlpNode> {
        &self.to_delete
    }

    pub(crate) fn to_delete_mut(&mut self) -> &mut Vec<RlpNode> {
        &mut self.to_delete
    }

    pub fn merge(mut self, other: Pending) -> Pending {
        self.to_delete.extend(other.to_delete);
        self.to_insert.extend(other.to_insert);

        self
    }

    pub fn replace(&mut self, src: RlpNode, dst: RlpNode, dst_node: TrieNode) -> RlpNode {
        self.delete(src);
        self.insert(dst.clone(), dst_node);
        dst
    }
}

pub trait ApplyOperations {
    fn apply_operations(&mut self, pending: Pending) -> anyhow::Result<()>;
}

impl<Storage, NodesTable> ApplyOperations
    for crate::merkle_patricia_trie::trie::Trie<Storage, NodesTable>
where
    Storage: StorageMutate<NodesTable>,
    NodesTable: Mappable<Key = RlpNode, Value = TrieNode, OwnedValue = TrieNode>,
{
    fn apply_operations(&mut self, pending: Pending) -> anyhow::Result<()> {
        for node in pending.to_delete {
            self.storage.remove(&node).map_err(|_e| {
                anyhow::anyhow!("Could not remove {:?} from storage", node)
            })?;
        }
        for (node, value) in pending.to_insert {
            self.storage.insert(&node, &value).map_err(|_e| {
                anyhow::anyhow!("Could not insert {:?} into storage", node)
            })?;
        }
        Ok(())
    }
}

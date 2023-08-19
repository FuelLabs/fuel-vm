use crate::{
    common::{
        path::ComparablePath,
        Bytes32,
    },
    sparse::{
        Node,
        Primitive,
    },
};
use fuel_storage::{
    Mappable,
    StorageMutate,
};

use crate::sparse::MerkleTreeError;
use core::iter;
use std::cmp::max;

pub(crate) struct Branch {
    pub bits: Bytes32,
    pub node: Node,
}

impl From<Node> for Branch {
    fn from(leaf: Node) -> Self {
        Self {
            bits: *leaf.leaf_key(),
            node: leaf,
        }
    }
}

pub(crate) fn merge_branches<Storage, Table>(
    storage: &mut Storage,
    mut left_branch: Branch,
    mut right_branch: Branch,
) -> Result<Branch, MerkleTreeError<Storage::Error>>
where
    Storage: StorageMutate<Table>,
    Table: Mappable<Key = Bytes32, Value = Primitive, OwnedValue = Primitive>,
{
    let branch = if left_branch.node.is_leaf() && right_branch.node.is_leaf() {
        let parent_depth = left_branch
            .node
            .common_path_length(&right_branch.node)
            .ok_or(MerkleTreeError::Overflow(
                "Cannot compute common path length of right branch".to_string(),
            ))?;
        let parent_height = Node::max_height()
            .and_then(|max_height| max_height.checked_sub(parent_depth))
            .ok_or(MerkleTreeError::Overflow(
                "Cannot subtract parent depth from max height".to_string(),
            ))? as u32;
        let node =
            Node::create_node(&left_branch.node, &right_branch.node, parent_height);
        Branch {
            bits: left_branch.bits,
            node,
        }
    } else {
        let ancestor_depth = left_branch
            .bits
            .common_path_length(&right_branch.bits)
            .ok_or(MerkleTreeError::Overflow(
                "Cannot compute common path length of right branch".to_string(),
            ))? as usize;
        let ancestor_height = Node::max_height()
            .and_then(|max_height| max_height.checked_sub(ancestor_depth))
            .ok_or(MerkleTreeError::Overflow(
                "Cannot subtract ancestor depth from max height".to_string(),
            ))?;
        if right_branch.node.is_node() {
            let mut current_node = right_branch.node;
            let path = right_branch.bits;
            let parent_height = (current_node.height() as usize).checked_add(1).ok_or(
                MerkleTreeError::Overflow(
                    "Cannot add 1 to current node height".to_string(),
                ),
            )?;
            let stale_depth = ancestor_height.checked_sub(parent_height).ok_or(
                MerkleTreeError::Overflow(
                    "Cannot subtract parent height from ancestor height".to_string(),
                ),
            )?;
            let placeholders = iter::repeat(Node::create_placeholder()).take(stale_depth);
            for placeholder in placeholders {
                current_node =
                    Node::create_node_on_path(&path, &current_node, &placeholder).ok_or(
                        MerkleTreeError::Overflow(
                            "Cannot create node on path".to_string(),
                        ),
                    )?;
                storage.insert(current_node.hash(), &current_node.as_ref().into())?;
            }
            right_branch.node = current_node;
        }
        if left_branch.node.is_node() {
            let mut current_node = left_branch.node;
            let path = left_branch.bits;
            let parent_height = (current_node.height() as usize).checked_add(1).ok_or(
                MerkleTreeError::Overflow(
                    "Cannot add 1 to current node height".to_string(),
                ),
            )?;
            let stale_depth = ancestor_height.checked_sub(parent_height).ok_or(
                MerkleTreeError::Overflow(
                    "Cannot subtract parent height from ancestor height".to_string(),
                ),
            )?;
            let placeholders = iter::repeat(Node::create_placeholder()).take(stale_depth);
            for placeholder in placeholders {
                current_node =
                    Node::create_node_on_path(&path, &current_node, &placeholder).ok_or(
                        MerkleTreeError::Overflow(
                            "Cannot create node on path".to_string(),
                        ),
                    )?;
                storage.insert(current_node.hash(), &current_node.as_ref().into())?;
            }
            left_branch.node = current_node;
        }
        let node = Node::create_node(
            &left_branch.node,
            &right_branch.node,
            ancestor_height as u32,
        );
        Branch {
            bits: left_branch.bits,
            node,
        }
    };
    storage.insert(branch.node.hash(), &branch.node.as_ref().into())?;
    Ok(branch)
}

use super::Node;
use crate::{
    common::{
        path::ComparablePath,
        Bytes32,
    },
    sparse::Primitive,
};
use fuel_storage::{
    Mappable,
    StorageMutate,
};

use core::iter;

pub(super) struct Branch {
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

pub(super) fn merge_branches<Storage, Table>(
    storage: &mut Storage,
    mut left_branch: Branch,
    mut right_branch: Branch,
) -> Result<Branch, Storage::Error>
where
    Storage: StorageMutate<Table>,
    Table: Mappable<Key = Bytes32, Value = Primitive, OwnedValue = Primitive>,
{
    let branch = if left_branch.node.is_leaf() && right_branch.node.is_leaf() {
        let parent_depth = left_branch.node.common_path_length(&right_branch.node);
        let parent_height = Node::max_height() - parent_depth;
        let node =
            Node::create_node(&left_branch.node, &right_branch.node, parent_height);
        Branch {
            bits: left_branch.bits,
            node,
        }
    } else {
        let ancestor_depth = left_branch.bits.common_path_length(&right_branch.bits);
        let ancestor_height = Node::max_height() - ancestor_depth;
        if right_branch.node.is_node() {
            let mut current_node = right_branch.node;
            let path = right_branch.bits;
            let parent_height = current_node.height() + 1;
            let stale_depth = ancestor_height - parent_height;
            let placeholders =
                iter::repeat(Node::create_placeholder()).take(stale_depth as usize);
            for placeholder in placeholders {
                current_node =
                    Node::create_node_on_path(&path, &current_node, &placeholder);
                storage.insert(current_node.hash(), &current_node.as_ref().into())?;
            }
            right_branch.node = current_node;
        }
        if left_branch.node.is_node() {
            let mut current_node = left_branch.node;
            let path = left_branch.bits;
            let parent_height = current_node.height() + 1;
            let stale_depth = ancestor_height - parent_height;
            let placeholders =
                iter::repeat(Node::create_placeholder()).take(stale_depth as usize);
            for placeholder in placeholders {
                current_node =
                    Node::create_node_on_path(&path, &current_node, &placeholder);
                storage.insert(current_node.hash(), &current_node.as_ref().into())?;
            }
            left_branch.node = current_node;
        }
        let node =
            Node::create_node(&left_branch.node, &right_branch.node, ancestor_height);
        Branch {
            bits: left_branch.bits,
            node,
        }
    };
    storage.insert(branch.node.hash(), &branch.node.as_ref().into())?;
    Ok(branch)
}

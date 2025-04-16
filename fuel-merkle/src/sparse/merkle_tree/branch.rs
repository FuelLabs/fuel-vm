use super::Node;
use crate::{
    common::{
        Bytes32,
        path::Path,
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
    #[allow(clippy::cast_possible_truncation)] // Key is 32 bytes, never truncates
    let ancestor_height = if left_branch.node.is_leaf() && right_branch.node.is_leaf() {
        let parent_depth = left_branch.node.common_path_length(&right_branch.node) as u32;
        #[allow(clippy::arithmetic_side_effects)] // common_path_length <= max_height
        let parent_height = Node::max_height() - parent_depth;
        parent_height
    } else {
        let ancestor_depth =
            left_branch.bits.common_path_length(&right_branch.bits) as u32;
        #[allow(clippy::arithmetic_side_effects)] // common_path_length <= max_height
        let ancestor_height = Node::max_height() - ancestor_depth;

        for branch in [&mut right_branch, &mut left_branch] {
            if branch.node.is_node() {
                let path = branch.bits;
                #[allow(clippy::arithmetic_side_effects)]
                // branch cannot be at max height
                let parent_height = branch.node.height() + 1;
                #[allow(clippy::arithmetic_side_effects)]
                // common_path_length <= max_height
                let stale_depth = ancestor_height - parent_height;
                let placeholders =
                    iter::repeat(Node::create_placeholder()).take(stale_depth as usize);
                for placeholder in placeholders {
                    branch.node =
                        Node::create_node_on_path(&path, &branch.node, &placeholder);
                    storage.insert(branch.node.hash(), &branch.node.as_ref().into())?;
                }
            }
        }
        ancestor_height
    };
    let node = Node::create_node(&left_branch.node, &right_branch.node, ancestor_height);
    storage.insert(node.hash(), &node.as_ref().into())?;
    Ok(Branch {
        bits: left_branch.bits,
        node,
    })
}

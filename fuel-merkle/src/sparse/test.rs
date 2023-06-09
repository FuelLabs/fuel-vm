use crate::{
    common::{path::ComparablePath, Bytes32},
    sparse::{Node, Primitive},
};
use fuel_storage::{Mappable, StorageMutate};

#[derive(Debug)]
pub struct NodesTable;

impl Mappable for NodesTable {
    type Key = Self::OwnedKey;
    type OwnedKey = Bytes32;
    type Value = Self::OwnedValue;
    type OwnedValue = Primitive;
}

pub struct Branch {
    pub bits: Bytes32,
    pub node: Node,
}

pub fn merge_branches<Storage, Table>(
    storage: &mut Storage,
    mut left_branch: Branch,
    mut right_branch: Branch,
) -> Result<Branch, Storage::Error>
where
    Storage: StorageMutate<Table>,
    Table: Mappable<Key = Bytes32, Value = Primitive, OwnedValue = Primitive>,
{
    let branch = if left_branch.node.is_leaf() && right_branch.node.is_leaf() {
        let path = left_branch.bits;
        let node = Node::create_node_on_path(&path, &left_branch.node, &right_branch.node);
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
            let parent_height = current_node.height() as usize + 1;
            let stale_depth = ancestor_height - parent_height;
            let placeholders = std::iter::repeat(Node::create_placeholder()).take(stale_depth);
            for placeholder in placeholders {
                current_node = Node::create_node_on_path(&path, &current_node, &placeholder);
                storage.insert(&current_node.hash(), &current_node.as_ref().into())?;
            }
            right_branch.node = current_node;
        }
        if left_branch.node.is_node() {
            let mut current_node = left_branch.node;
            let path = left_branch.bits;
            let parent_height = current_node.height() as usize + 1;
            let stale_depth = ancestor_height - parent_height;
            let placeholders = std::iter::repeat(Node::create_placeholder()).take(stale_depth);
            for placeholder in placeholders {
                current_node = Node::create_node_on_path(&path, &current_node, &placeholder);
                storage.insert(&current_node.hash(), &current_node.as_ref().into())?;
            }
            left_branch.node = current_node;
        }
        let node = Node::create_node_on_path(&left_branch.bits, &left_branch.node, &right_branch.node);
        Branch {
            bits: left_branch.bits,
            node,
        }
    };

    storage.insert(&branch.node.hash(), &branch.node.as_ref().into())?;
    Ok(branch)
}

pub fn update_set_v2<'a, I, Storage, Table>(storage: &mut Storage, set: I) -> Result<Bytes32, Storage::Error>
where
    I: IntoIterator<Item = (&'a Bytes32, &'a Bytes32)>,
    Storage: StorageMutate<Table>,
    Table: Mappable<Key = Bytes32, Value = Primitive, OwnedValue = Primitive>,
{
    let mut upcoming = set
        .into_iter()
        .map(|(key, data)| {
            let leaf_node = Node::create_leaf(key, data);
            storage.insert(&leaf_node.hash(), &leaf_node.as_ref().into())?;
            storage.insert(leaf_node.leaf_key(), &leaf_node.as_ref().into())?;

            Ok(Branch {
                bits: *leaf_node.leaf_key(),
                node: leaf_node,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut leaves_stack = Vec::<Branch>::new();
    let mut differences = vec![];
    let mut second_differences = vec![];

    while let Some(leaf) = upcoming.pop() {
        if let Some(prev_leaf) = leaves_stack.last() {
            let difference = leaf.node.common_path_length(&prev_leaf.node) as i64;
            if let Some(prev_diff) = differences.last() {
                let second_difference = prev_diff - difference;
                second_differences.push(second_difference);
            }
            differences.push(difference);
        }

        leaves_stack.push(leaf);

        while let Some(second_difference) = second_differences.pop() {
            if second_difference > 0 {
                // Safety:
                // The presence of a second difference guarantees at least three
                // leaves.
                let n2 = leaves_stack.pop().unwrap();
                let n1 = leaves_stack.pop().unwrap();
                let n0 = leaves_stack.pop().unwrap();
                let merged = merge_branches(storage, n0, n1)?;
                leaves_stack.push(merged);
                leaves_stack.push(n2);

                // Remove sd0
                second_differences.pop();

                // Remove d1
                let d2 = differences.pop().unwrap();
                let _d1 = differences.pop().unwrap();
                if let Some(prev_diff) = differences.last() {
                    let second_difference = prev_diff - d2;
                    second_differences.push(second_difference);
                }
                differences.push(d2);
            }
        }
    }

    let top = {
        let mut node = leaves_stack.pop().expect("Leaves stack must have at least 1 element.");
        while let Some(next) = leaves_stack.pop() {
            node = merge_branches(storage, next, node)?;
        }
        node
    };

    let mut node = top.node;
    let path = top.bits;
    let height = node.height() as usize;
    let depth = Node::max_height() - height;
    let placeholders = std::iter::repeat(Node::create_placeholder()).take(depth);
    for placeholder in placeholders {
        node = Node::create_node_on_path(&path, &node, &placeholder);
        storage.insert(&node.hash(), &node.as_ref().into())?;
    }

    Ok(node.hash())
}

#[cfg(test)]
mod test {
    use rand::Rng;
    use std::collections::BTreeMap;

    use crate::common::StorageMap;
    use crate::sparse::MerkleTree;

    use super::*;

    type Storage = StorageMap<NodesTable>;

    fn random_bytes32<R>(rng: &mut R) -> Bytes32
    where
        R: Rng + ?Sized,
    {
        let mut bytes = [0u8; 32];
        rng.fill(bytes.as_mut());
        bytes
    }

    #[test]
    fn test_update_set() {
        let rng = &mut rand::thread_rng();
        let gen = || Some((random_bytes32(rng), random_bytes32(rng)));
        let data = std::iter::from_fn(gen).take(100000).collect::<Vec<_>>();
        let input: BTreeMap<Bytes32, Bytes32> = BTreeMap::from_iter(data.into_iter());

        let storage = Storage::new();
        let mut tree = MerkleTree::<NodesTable, Storage>::new(storage);
        tree.update_set(&input).unwrap();
        let expected_root = tree.root();

        let mut storage = Storage::new();
        let root = update_set_v2(&mut storage, &input).unwrap();

        assert_eq!(expected_root, root);
    }
}

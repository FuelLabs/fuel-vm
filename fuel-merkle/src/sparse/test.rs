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
    let mut leaves = set
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

    let mut nodes = Vec::<Branch>::new();
    let mut proximities = Vec::<i64>::new();

    while let Some(next) = leaves.pop() {
        if let Some(current) = nodes.last() {
            let proximity = current.node.common_path_length(&next.node) as i64;
            if let Some(previous_proximity) = proximities.last() {
                let mut difference = previous_proximity - proximity;
                while difference > 0 {
                    // A positive difference in proximity means that the current
                    // node is closer to its right neighbor than its left
                    // neighbor. We now merge the current node with its right
                    // neighbor.
                    let right = nodes.pop().unwrap();
                    let current = nodes.pop().unwrap();
                    let merged = merge_branches(storage, current, right)?;
                    nodes.push(merged);

                    // Now that the current node and its right neighbour are
                    // merged, the distance between them has collapsed and their
                    // proximity is no longer needed.
                    proximities.pop();

                    // If the merged node is now adjacent to another node, we
                    // calculate the difference in proximities to determine if
                    // we must merge again.
                    if let Some(previous_proximity) = proximities.last() {
                        difference = previous_proximity - proximity;
                    } else {
                        break;
                    }
                }
            }
            proximities.push(proximity);
        }
        nodes.push(next);
    }

    let top = {
        let mut node = nodes.pop().expect("Nodes stack must have at least 1 element.");
        while let Some(next) = nodes.pop() {
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
        let data = std::iter::from_fn(gen).take(100_000).collect::<Vec<_>>();
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

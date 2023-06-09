use crate::{
    common::Bytes32,
    sparse::{
        branch::{merge_branches, Branch},
        Node, Primitive,
    },
};
use fuel_storage::{Mappable, StorageMutate};

pub fn update_set_v2<'a, I, Storage, Table>(storage: &mut Storage, set: I) -> Result<Bytes32, Storage::Error>
where
    I: IntoIterator<Item = (&'a Bytes32, &'a Bytes32)>,
    Storage: StorageMutate<Table>,
    Table: Mappable<Key = Bytes32, Value = Primitive, OwnedValue = Primitive>,
{
    let mut leaves = set
        .into_iter()
        .map(|(key, data)| Node::create_leaf(key, data))
        .map(|leaf_node| {
            storage.insert(&leaf_node.hash(), &leaf_node.as_ref().into())?;
            Ok(Branch {
                bits: *leaf_node.leaf_key(),
                node: leaf_node,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    leaves.sort_by(|a, b| a.bits.cmp(&b.bits));

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
                    let current = nodes.pop().unwrap();
                    let right = nodes.pop().unwrap();
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

    use crate::common::StorageMap;
    use crate::sparse::MerkleTree;

    use super::*;

    #[derive(Debug)]
    pub struct NodesTable;

    impl Mappable for NodesTable {
        type Key = Self::OwnedKey;
        type OwnedKey = Bytes32;
        type Value = Self::OwnedValue;
        type OwnedValue = Primitive;
    }

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
        let input = std::iter::from_fn(gen).take(250_000).collect::<Vec<_>>();

        let storage = Storage::new();
        let tree = MerkleTree::<NodesTable, Storage>::from_set::<&Vec<_>, &Bytes32>(storage, input.as_ref()).unwrap();
        let expected_root = tree.root();

        // let mut storage = Storage::new();
        // let root = update_set_v2(&mut storage, input.as_ref()).unwrap();
        //
        // assert_eq!(expected_root, root);
    }
}

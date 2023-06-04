use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fuel_merkle::common::{Bytes32, StorageMap};
use fuel_merkle::sparse::{MerkleTree, Node, Primitive};
use fuel_storage::{Mappable, StorageMutate};
use rand::Rng;
use std::collections::BTreeMap;

fn random_bytes32<R>(rng: &mut R) -> Bytes32
where
    R: Rng + ?Sized,
{
    let mut bytes = [0u8; 32];
    rng.fill(bytes.as_mut());
    bytes
}

#[derive(Debug)]
pub struct NodesTable;

impl Mappable for NodesTable {
    type Key = Self::OwnedKey;
    type OwnedKey = Bytes32;
    type Value = Self::OwnedValue;
    type OwnedValue = Primitive;
}

type Storage = StorageMap<NodesTable>;

trait Prefix {
    fn common_prefix(&self, other: &Self) -> usize;
}

impl Prefix for Bytes32 {
    fn common_prefix(&self, other: &Self) -> usize {
        for i in 0..self.len() {
            if self[i] == other[i] {
                continue;
            } else {
                for k in 0..8 {
                    let bit = 1 << (7 - k);
                    if self[i] & bit == other[i] & bit {
                        continue;
                    } else {
                        return 8 * i + k;
                    }
                }
            }
        }
        256
    }
}

struct Branch {
    bits: Bytes32,
    node: Node,
}

// Naive update set: Updates the Merkle tree sequentially.
// This is the baseline. Performance improvements to the Sparse Merkle Tree's
// update_set must demonstrate an increase in speed relative to this baseline.
pub fn update_set_baseline<'a, I, Storage>(storage: &mut Storage, set: I) -> Result<Bytes32, Storage::Error>
where
    I: IntoIterator<Item = (&'a Bytes32, &'a Bytes32)>,
    Storage: StorageMutate<NodesTable>,
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
    let mut stack: Vec<Branch> = Vec::with_capacity(upcoming.len());

    while !upcoming.is_empty() {
        let current = upcoming.pop().expect("We checked that above");

        match (upcoming.pop(), stack.pop()) {
            (Some(left), Some(right)) => {
                let left_cur = left.bits.common_prefix(&current.bits);
                let cur_right = current.bits.common_prefix(&right.bits);

                if left_cur < cur_right {
                    let branch = merge_branches(storage, current, right)?;
                    upcoming.push(left);
                    upcoming.push(branch);
                } else {
                    upcoming.push(left);
                    stack.push(right);
                    stack.push(current);
                }
            }
            (Some(left), None) => {
                stack.push(current);
                upcoming.push(left);
            }
            (None, Some(right)) => {
                let branch = merge_branches(storage, current, right)?;
                upcoming.push(branch);
            }
            (None, None) => {
                stack.push(current);
            }
        }
    }

    assert_eq!(stack.len(), 1);

    Ok(stack[0].node.hash())
}

fn merge_branches<Storage>(
    storage: &mut Storage,
    mut left_branch: Branch,
    mut right_branch: Branch,
) -> Result<Branch, Storage::Error>
where
    Storage: StorageMutate<NodesTable>,
{
    use fuel_merkle::common::msb::Bit;
    use fuel_merkle::common::msb::Msb;

    let branch = if left_branch.node.is_leaf() && right_branch.node.is_leaf() {
        let parent_depth = left_branch.bits.common_prefix(&right_branch.bits);
        let parent_height = (Node::max_height() - parent_depth) as u32;
        let node = Node::create_node(&left_branch.node, &right_branch.node, parent_height);
        Branch {
            bits: left_branch.bits,
            node,
        }
    } else {
        let parent_depth = left_branch.bits.common_prefix(&right_branch.bits);
        let parent_height = (Node::max_height() - parent_depth) as u32;

        if right_branch.node.is_node() {
            let start_height = right_branch.node.height() + 1;
            for height in start_height..parent_height {
                let byte_index = Node::max_height() - height as usize;

                match right_branch.bits.get_bit_at_index_from_msb(byte_index).unwrap() {
                    Bit::_0 => {
                        let node = Node::create_node(&right_branch.node, &Node::create_placeholder(), height);

                        right_branch = Branch {
                            bits: right_branch.bits,
                            node,
                        };
                    }
                    Bit::_1 => {
                        let node = Node::create_node(&Node::create_placeholder(), &right_branch.node, height);

                        right_branch = Branch {
                            bits: right_branch.bits,
                            node,
                        };
                    }
                }
                storage.insert(&right_branch.node.hash(), &right_branch.node.as_ref().into())?;
            }
        }

        if left_branch.node.is_node() {
            let start_height = left_branch.node.height() + 1;
            for height in start_height..parent_height {
                let byte_index = Node::max_height() - height as usize;

                match left_branch.bits.get_bit_at_index_from_msb(byte_index).unwrap() {
                    Bit::_0 => {
                        let node = Node::create_node(&left_branch.node, &Node::create_placeholder(), height);

                        left_branch = Branch {
                            bits: left_branch.bits,
                            node,
                        };
                    }
                    Bit::_1 => {
                        let node = Node::create_node(&Node::create_placeholder(), &left_branch.node, height);

                        left_branch = Branch {
                            bits: left_branch.bits,
                            node,
                        };
                    }
                }
                storage.insert(&left_branch.node.hash(), &left_branch.node.as_ref().into())?;
            }
        }

        let node = Node::create_node(&left_branch.node, &right_branch.node, parent_height);
        Branch {
            bits: left_branch.bits,
            node,
        }
    };

    storage.insert(&branch.node.hash(), &branch.node.as_ref().into())?;
    Ok(branch)
}

fn sparse_merkle_tree(c: &mut Criterion) {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let rng = &mut StdRng::seed_from_u64(8586);
    let gen = || Some((random_bytes32(rng), random_bytes32(rng)));
    let data = std::iter::from_fn(gen).take(64000).collect::<Vec<_>>();
    let input: BTreeMap<Bytes32, Bytes32> = BTreeMap::from_iter(data.into_iter());

    let storage = Storage::new();
    let mut tree = MerkleTree::<NodesTable, Storage>::new(storage);
    tree.update_set(black_box(&input)).unwrap();
    let mut storage = Storage::new();
    let baseline_root = update_set_baseline(black_box(&mut storage), black_box(&input)).unwrap();

    assert_eq!(tree.root(), baseline_root);

    let mut group_update = c.benchmark_group("update");

    group_update.bench_with_input("update-set-baseline", &input, |b, input| {
        let mut storage = Storage::new();
        b.iter(|| update_set_baseline(black_box(&mut storage), black_box(input)));
    });

    group_update.bench_with_input("update-set", &input, |b, input| {
        let storage = Storage::new();
        let mut tree = MerkleTree::<NodesTable, Storage>::new(storage);
        b.iter(|| tree.update_set(black_box(input)));
    });

    group_update.finish();
}

criterion_group!(benches, sparse_merkle_tree);
criterion_main!(benches);

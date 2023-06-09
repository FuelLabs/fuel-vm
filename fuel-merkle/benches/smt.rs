use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fuel_merkle::{
    common::{path::ComparablePath, Bytes32, StorageMap},
    sparse::{
        test::{merge_branches, update_set_v2, Branch},
        MerkleTree, Node, Primitive,
    },
};
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
                let left_cur = left.bits.common_path_length(&current.bits);
                let cur_right = current.bits.common_path_length(&right.bits);

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
                // return Ok(current.node.hash());
            }
        }
    }

    assert_eq!(stack.len(), 1);

    let top = stack.pop().unwrap();
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

fn sparse_merkle_tree(c: &mut Criterion) {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let rng = &mut StdRng::seed_from_u64(8586);
    let gen = || Some((random_bytes32(rng), random_bytes32(rng)));
    let data = std::iter::from_fn(gen).take(1_000_000).collect::<Vec<_>>();

    // let l0 = Bytes32::default(); // left, left, left, left left, ...
    //
    // let mut l1 = Bytes32::default();
    // l1[0..1].copy_from_slice(&[0b01000000]); // left, right, left, left, left, ...
    //
    // let mut l2 = Bytes32::default();
    // l2[0..1].copy_from_slice(&[0b01100000]); // left, right, right, ...
    //
    // let mut l3 = Bytes32::default();
    // l3[0..1].copy_from_slice(&[0b01001000]); // left, right, left, left, right, ...
    //
    // let data = [
    //     (l0, random_bytes32(rng)),
    //     (l1, random_bytes32(rng)),
    //     (l2, random_bytes32(rng)),
    //     (l3, random_bytes32(rng)),
    // ];

    let input: BTreeMap<Bytes32, Bytes32> = BTreeMap::from_iter(data.into_iter());

    // let storage = Storage::new();
    // let mut tree = MerkleTree::<NodesTable, Storage>::new(storage);
    // tree.update_set(black_box(&input)).unwrap();
    //
    // let mut storage = Storage::new();
    // let baseline_root = update_set_baseline(black_box(&mut storage), black_box(&input)).unwrap();
    //
    // assert_eq!(tree.root(), baseline_root);
    //
    // let mut storage = Storage::new();
    // let v2_root = update_set_v2(black_box(&mut storage), black_box(&input)).unwrap();
    // assert_eq!(tree.root(), v2_root);

    let mut group_update = c.benchmark_group("update");

    group_update.bench_with_input("update-set-baseline", &input, |b, input| {
        let mut storage = Storage::new();
        b.iter(|| update_set_baseline(black_box(&mut storage), black_box(input)));
    });

    group_update.bench_with_input("update-set-v2", &input, |b, input| {
        let mut storage = Storage::new();
        b.iter(|| update_set_v2(black_box(&mut storage), black_box(input)));
    });

    // group_update.bench_with_input("update-set", &input, |b, input| {
    //     let storage = Storage::new();
    //     let mut tree = MerkleTree::<NodesTable, Storage>::new(storage);
    //     b.iter(|| tree.update_set(black_box(input)));
    // });

    group_update.finish();
}

criterion_group!(benches, sparse_merkle_tree);
criterion_main!(benches);

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fuel_merkle::{
    common::{path::ComparablePath, Bytes32, StorageMap},
    sparse::{
        branch::{merge_branches, Branch},
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
    let mut tree = MerkleTree::new(storage);
    for (key, data) in set.into_iter() {
        tree.update(key, data)?;
    }
    Ok(tree.root())
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

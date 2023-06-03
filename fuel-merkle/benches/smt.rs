use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fuel_merkle::common::{Bytes32, StorageMap};
use fuel_merkle::sparse::{MerkleTree, MerkleTreeError, Primitive};
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
pub fn update_set_baseline<'a, I, Storage>(
    tree: &mut MerkleTree<NodesTable, Storage>,
    set: I,
) -> Result<(), MerkleTreeError<MerkleTreeError<Storage::Error>>>
where
    I: IntoIterator<Item = (&'a Bytes32, &'a Bytes32)>,
    Storage: StorageMutate<NodesTable>,
{
    let iter = set.into_iter();
    for (key, data) in iter {
        tree.update(key, data.as_ref())?;
    }

    Ok(())
}

fn sparse_merkle_tree(c: &mut Criterion) {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let rng = &mut StdRng::seed_from_u64(8586);
    let gen = || Some((random_bytes32(rng), random_bytes32(rng)));
    let data = std::iter::from_fn(gen).take(10000).collect::<Vec<_>>();
    let input: BTreeMap<Bytes32, Bytes32> = BTreeMap::from_iter(data.into_iter());

    let mut group_update = c.benchmark_group("update");

    group_update.bench_with_input("update-set-baseline", &input, |b, input| {
        let storage = Storage::new();
        let mut tree = MerkleTree::<NodesTable, Storage>::new(storage);
        b.iter(|| update_set_baseline(black_box(&mut tree), black_box(input)));
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

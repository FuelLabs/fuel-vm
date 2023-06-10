use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fuel_merkle::{
    common::{Bytes32, StorageMap},
    sparse::{MerkleTree, MerkleTreeError, Primitive},
};
use fuel_storage::{Mappable, StorageInspect};
use rand::Rng;

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
type StorageError = <Storage as StorageInspect<NodesTable>>::Error;

// Naive update set: Updates the Merkle tree sequentially.
// This is the baseline. Performance improvements to the Sparse Merkle Tree's
// update_set must demonstrate an increase in speed relative to this baseline.
pub fn baseline_root<I, D>(set: I) -> Result<Bytes32, MerkleTreeError<StorageError>>
where
    I: Iterator<Item = (Bytes32, D)>,
    D: AsRef<[u8]>,
{
    let storage = Storage::new();
    let mut tree = MerkleTree::new(storage);
    for (key, data) in set {
        tree.update(&key, data.as_ref())?;
    }
    let root = tree.root();
    Ok(root)
}

pub fn subject_root<I, D>(set: I) -> Result<Bytes32, MerkleTreeError<StorageError>>
where
    I: Iterator<Item = (Bytes32, D)>,
    D: AsRef<[u8]>,
{
    let storage = Storage::new();
    let tree = MerkleTree::from_set(storage, set)?;
    let root = tree.root();
    Ok(root)
}

fn sparse_merkle_tree(c: &mut Criterion) {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let rng = &mut StdRng::seed_from_u64(8586);
    let gen = || Some((random_bytes32(rng), random_bytes32(rng)));
    let data = std::iter::from_fn(gen).take(50_000).collect::<Vec<_>>();

    let expected_root = baseline_root(data.clone().into_iter()).unwrap();
    let root = subject_root(data.clone().into_iter()).unwrap();

    assert_eq!(expected_root, root);

    let mut group_update = c.benchmark_group("from-set");

    group_update.bench_with_input("from-set-baseline", &data, |b, data| {
        b.iter(|| baseline_root(black_box(data.clone().into_iter())));
    });

    group_update.bench_with_input("from-set", &data, |b, data| {
        b.iter(|| subject_root(black_box(data.clone().into_iter())));
    });

    group_update.finish();
}

criterion_group!(benches, sparse_merkle_tree);
criterion_main!(benches);

use alloy_trie::nodes::RlpNode;
use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    Criterion,
};
use fuel_merkle::{
    common::Bytes32,
    merkle_patricia_trie,
    sparse::{
        in_memory,
        MerkleTreeKey,
    },
};
use rand::Rng;

fn random_bytes32<R>(rng: &mut R) -> Bytes32
where
    R: Rng + ?Sized,
{
    let mut bytes = [0u8; 32];
    rng.fill(bytes.as_mut());
    bytes
}

// Naive update set: Updates the Merkle tree sequentially.
// This is the baseline. Performance improvements to the Sparse Merkle Tree's
// update_set must demonstrate an increase in speed relative to this baseline.
pub fn baseline_root<I, D>(set: I) -> Bytes32
where
    I: Iterator<Item = (MerkleTreeKey, D)>,
    D: AsRef<[u8]>,
{
    let mut tree = in_memory::MerkleTree::new();
    for (key, data) in set {
        tree.update(key, data.as_ref());
    }
    tree.root()
}

pub fn baseline_merkle_patricia_trie_root<I>(set: I) -> RlpNode
where
    I: Iterator<Item = (MerkleTreeKey, Bytes32)>,
{
    let mut trie = merkle_patricia_trie::in_memory::MerklePatriciaTrie::new();
    for (key, data) in set {
        trie.update(key.into(), &data);
    }
    trie.root()
}

pub fn subject_root<I, D>(set: I) -> Bytes32
where
    I: Iterator<Item = (MerkleTreeKey, D)>,
    D: AsRef<[u8]>,
{
    let tree = in_memory::MerkleTree::from_set(set);
    tree.root()
}

pub fn subject_only_root<I, D>(set: I) -> Bytes32
where
    I: Iterator<Item = (MerkleTreeKey, D)>,
    D: AsRef<[u8]>,
{
    in_memory::MerkleTree::root_from_set(set)
}

pub fn subject_nodes<I, D>(set: I) -> Bytes32
where
    I: Iterator<Item = (MerkleTreeKey, D)>,
    D: AsRef<[u8]>,
{
    in_memory::MerkleTree::nodes_from_set(set).0
}

fn sparse_merkle_tree(c: &mut Criterion) {
    use rand::{
        rngs::StdRng,
        SeedableRng,
    };

    let rng = &mut StdRng::seed_from_u64(8586);
    let gen = || Some((MerkleTreeKey::new(random_bytes32(rng)), random_bytes32(rng)));
    let data = core::iter::from_fn(gen).take(100_000).collect::<Vec<_>>();

    let expected_root = baseline_root(data.clone().into_iter());
    let root = subject_root(data.clone().into_iter());
    let only_root = subject_only_root(data.clone().into_iter());
    let nodes_root = subject_nodes(data.clone().into_iter());

    assert_eq!(expected_root, root);
    assert_eq!(expected_root, only_root);
    assert_eq!(expected_root, nodes_root);

    let mut group_update = c.benchmark_group("from-set");

    group_update.bench_with_input(
        "from-set-baseline-merkle-patricia-trie",
        &data,
        |b, data| {
            b.iter(|| {
                baseline_merkle_patricia_trie_root::<_>(black_box(
                    data.clone().into_iter(),
                ))
            });
        },
    );

    group_update.bench_with_input("from-set-baseline", &data, |b, data| {
        b.iter(|| baseline_root(black_box(data.clone().into_iter())));
    });
    group_update.bench_with_input("root-from-set", &data, |b, data| {
        b.iter(|| subject_only_root(black_box(data.clone().into_iter())));
    });

    group_update.bench_with_input("nodes-from-set", &data, |b, data| {
        b.iter(|| subject_nodes(black_box(data.clone().into_iter())));
    });

    group_update.bench_with_input("from-set", &data, |b, data| {
        b.iter(|| subject_root(black_box(data.clone().into_iter())));
    });

    group_update.finish();
}

criterion_group!(benches, sparse_merkle_tree);
criterion_main!(benches);

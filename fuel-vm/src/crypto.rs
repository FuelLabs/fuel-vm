//! Crypto implementations for the instructions

use fuel_merkle::binary::root_calculator::MerkleRootCalculator as MerkleTree;
use fuel_types::Bytes32;

/// Calculate a binary merkle root with in-memory storage
pub fn ephemeral_merkle_root<L, I>(leaves: I) -> Bytes32
where
    L: AsRef<[u8]>,
    I: Iterator<Item = L> + ExactSizeIterator,
{
    let mut tree = MerkleTree::new();
    leaves.for_each(|l| tree.push(l.as_ref(), None));
    tree.root().into()
}

#[test]
#[cfg(feature = "random")]
fn ephemeral_merkle_root_returns_the_expected_root() {
    use fuel_crypto::Hasher;
    use rand::{
        rngs::StdRng,
        Rng,
        SeedableRng,
    };

    use alloc::{
        vec,
        vec::Vec,
    };

    use crate::prelude::*;

    let mut rng = StdRng::seed_from_u64(2322u64);

    const LEAF_PREFIX: u8 = 0x00;
    const NODE_PREFIX: u8 = 0x01;

    // Test for 0 leaves
    //
    // Expected root is `h()`
    let empty: Vec<Address> = vec![];

    let root = ephemeral_merkle_root(empty.iter());
    let empty = Hasher::default().digest();

    assert_eq!(empty, root);

    // Test for 5 leaves
    let a: Address = rng.gen();
    let b: Address = rng.gen();
    let c: Address = rng.gen();
    let d: Address = rng.gen();
    let e: Address = rng.gen();

    let initial = [a, b, c, d, e];

    let a = Hasher::default().chain([LEAF_PREFIX]).chain(a).digest();
    let b = Hasher::default().chain([LEAF_PREFIX]).chain(b).digest();
    let c = Hasher::default().chain([LEAF_PREFIX]).chain(c).digest();
    let d = Hasher::default().chain([LEAF_PREFIX]).chain(d).digest();
    let e = Hasher::default().chain([LEAF_PREFIX]).chain(e).digest();

    let a = Hasher::default()
        .chain([NODE_PREFIX])
        .extend_chain([a, b])
        .digest();
    let b = Hasher::default()
        .chain([NODE_PREFIX])
        .extend_chain([c, d])
        .digest();
    let c = e;

    let a = Hasher::default()
        .chain([NODE_PREFIX])
        .extend_chain([a, b])
        .digest();
    let b = c;

    let root = Hasher::default()
        .chain([NODE_PREFIX])
        .extend_chain([a, b])
        .digest();
    let root_p = ephemeral_merkle_root(initial.iter());

    assert_eq!(root, root_p);
}

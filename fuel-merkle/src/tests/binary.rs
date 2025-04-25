extern crate core;

use rand::{
    Rng,
    seq::IteratorRandom,
    thread_rng,
};
use sha2::{
    Digest,
    Sha256,
};

use crate::{
    binary::{
        MerkleTree,
        Primitive,
    },
    common::{
        Bytes32,
        StorageMap,
    },
};
use fuel_merkle_test_helpers::binary::MerkleTree as ReferenceMerkleTree;
use fuel_storage::Mappable;

struct TestTable;

impl Mappable for TestTable {
    type Key = Self::OwnedKey;
    type OwnedKey = u64;
    type OwnedValue = Primitive;
    type Value = Self::OwnedValue;
}

// During test setup, we randomly sample the pool of test data to generate the
// leaf set for the test and reference Merkle trees. Each test consists of a
// number of iterations, and at each iteration we specify a larger sample size.
const SAMPLE_SIZES: &[usize] = &[
    1, 2, 5, 7, 8, 9, 64, 500, 512, 1000, 1024, 2048, 5000, 10000,
];

fn sum(data: &[u8]) -> Bytes32 {
    let mut hash = Sha256::new();
    hash.update(data);
    hash.finalize().into()
}

#[test]
fn test_roots() {
    let test_data_count = 2u64.pow(16);
    let test_data = (0..test_data_count)
        .map(|i| sum(&i.to_be_bytes()))
        .collect::<Vec<Bytes32>>();

    let mut rng = thread_rng();
    for samples in SAMPLE_SIZES {
        let sample_data = test_data
            .iter()
            .cloned()
            .choose_multiple(&mut rng, *samples);

        let expected_root = {
            let mut reference_tree = ReferenceMerkleTree::new();
            for datum in sample_data.iter() {
                reference_tree.push(datum);
            }
            reference_tree.root()
        };

        let root = {
            let storage = StorageMap::<TestTable>::new();
            let mut test_tree = MerkleTree::new(storage);
            for datum in sample_data.iter() {
                test_tree.push(datum).unwrap();
            }
            test_tree.root()
        };

        assert_eq!(root, expected_root);
    }
}

#[test]
fn test_prove() {
    let test_data_count = 2u64.pow(16);
    let test_data = (0..test_data_count)
        .map(|i| sum(&i.to_be_bytes()))
        .collect::<Vec<Bytes32>>();

    let mut rng = thread_rng();
    for samples in SAMPLE_SIZES {
        let sample_data = test_data
            .iter()
            .cloned()
            .choose_multiple(&mut rng, *samples);
        let index = rng.gen_range(0..*samples) as u64;

        let expected_proof = {
            let mut reference_tree = ReferenceMerkleTree::new();
            reference_tree.set_proof_index(index);
            for datum in sample_data.iter() {
                reference_tree.push(datum);
            }
            reference_tree.prove()
        };

        let proof = {
            let storage = StorageMap::<TestTable>::new();
            let mut test_tree = MerkleTree::new(storage);
            for datum in sample_data.iter() {
                test_tree.push(datum).unwrap();
            }
            test_tree.prove(index).unwrap()
        };

        assert_eq!(proof, expected_proof);
    }
}

#[test]
fn test_load() {
    let test_data_count = 2u64.pow(16);
    let test_data = (0..test_data_count)
        .map(|i| sum(&i.to_be_bytes()))
        .collect::<Vec<Bytes32>>();

    let mut rng = thread_rng();
    for samples in SAMPLE_SIZES {
        let sample_data = test_data
            .iter()
            .cloned()
            .choose_multiple(&mut rng, *samples);

        let mut storage = StorageMap::<TestTable>::new();

        let expected_root = {
            let mut reference_tree = MerkleTree::new(&mut storage);
            for datum in sample_data.iter() {
                reference_tree.push(datum).unwrap();
            }
            reference_tree.root()
        };

        let root = {
            let leaves_count = sample_data.len() as u64;
            let test_tree = MerkleTree::load(&mut storage, leaves_count).unwrap();
            test_tree.root()
        };

        assert_eq!(root, expected_root);
    }
}

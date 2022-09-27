extern crate core;

use rand::{seq::IteratorRandom, thread_rng, Rng};
use sha2::{Digest, Sha256};

use fuel_merkle::{binary::in_memory::MerkleTree, common::Bytes32};
use fuel_merkle_test_helpers::binary::MerkleTree as ReferenceMerkleTree;

/// ReferenceTest
///
/// A ReferenceTest instantiates an instance of the system under test as well as
/// an instance of a reference system. Executing the ReferenceTest is done by
/// calling one of its `compare` methods: the test calls the same method on both
/// the SUT and the reference and compares the results.
///
struct ReferenceTest {
    test_tree: MerkleTree,
    reference_tree: ReferenceMerkleTree,
    proof_index: u64,
}

impl ReferenceTest {
    pub fn new() -> Self {
        Self {
            test_tree: MerkleTree::new(),
            reference_tree: ReferenceMerkleTree::new(),
            proof_index: Default::default(),
        }
    }

    pub fn set_proof_index(&mut self, index: u64) {
        self.proof_index = index;
        self.reference_tree.set_proof_index(index);
    }

    pub fn provision(&mut self, data: &[Bytes32]) {
        for datum in data {
            self.test_tree.push(datum);
            self.reference_tree.push(datum);
        }
    }

    pub fn compare_roots(mut self) {
        let root = self.test_tree.root();
        let expected_root = self.reference_tree.root();
        assert_eq!(root, expected_root);
    }

    pub fn compare_proofs(mut self) {
        let proof = self.test_tree.prove(self.proof_index).unwrap();
        let expected_proof = self.reference_tree.prove();
        assert_eq!(proof, expected_proof);
    }
}

// During test setup, we randomly sample the pool of test data to generate the
// leaf set for the test and reference Merkle trees. Each test consists of a
// number of iterations, and at each iteration we specify a larger sample size.
const SAMPLE_SIZES: [usize; 10] = [1, 2, 5, 8, 64, 500, 1000, 2048, 5000, 10000];

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
        let mut test = ReferenceTest::new();
        let sample_data = test_data.iter().cloned().choose_multiple(&mut rng, samples);
        test.provision(sample_data.as_slice());
        test.compare_roots();
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
        let mut test = ReferenceTest::new();
        let sample_data = test_data.iter().cloned().choose_multiple(&mut rng, samples);
        let index = rng.gen_range(0..samples) as u64;
        test.set_proof_index(index);
        test.provision(sample_data.as_slice());
        test.compare_proofs();
    }
}

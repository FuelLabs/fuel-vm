use fuel_merkle::{
    binary::in_memory::MerkleTree,
    common::Bytes32,
};
use fuel_merkle_test_helpers::data::{
    EncodedValue,
    Encoding,
    binary::ProofTest,
};

use digest::Digest;
use function_name::named;
use rand::seq::IteratorRandom;
use rand_pcg::Pcg64;
use rand_seeder::Seeder;
use sha2::Sha256;

type Hash = Sha256;

pub fn sum(data: &[u8]) -> Bytes32 {
    let mut hash = Hash::new();
    hash.update(data);
    hash.finalize().into()
}

fn generate_test(
    name: String,
    function_name: String,
    description: String,
    sample_data: &[Bytes32],
    proof_index: u64,
) -> ProofTest {
    let (root, proof_set) = {
        let mut test_tree = MerkleTree::new();
        for datum in sample_data.iter() {
            test_tree.push(datum);
        }
        // SAFETY: prove(i) is guaranteed to return a valid proof if the proof
        // index is within the range of valid leaves. proof_index will always
        // be selected from this range.
        test_tree.prove(proof_index).unwrap()
    };
    let data = sample_data[proof_index as usize];

    // SAFETY: All EncodedValues are specified with a valid encoding.
    let encoded_root = EncodedValue::from_raw(root, Encoding::Hex);
    let encoded_data = EncodedValue::from_raw(data, Encoding::Hex);
    let encoded_proof_set = proof_set
        .iter()
        .map(|v| EncodedValue::from_raw(v, Encoding::Hex))
        .collect::<Vec<_>>();
    let num_leaves = sample_data.len() as u64;

    ProofTest {
        name,
        function_name,
        description,
        root: encoded_root,
        data: encoded_data,
        proof_set: encoded_proof_set,
        proof_index,
        num_leaves,
        expected_verification: true,
    }
}

fn write_test(test: &ProofTest) {
    let yaml = serde_yaml::to_string(test).expect("Unable to serialize test!");
    let dir = "./fuel-merkle/tests-data-binary/fixtures";
    std::fs::write(format!("{}/{}.yaml", dir, test.name), yaml)
        .expect("Unable to write file!");
}

#[named]
fn generate_test_10_leaves_index_4(test_data: &[Bytes32]) -> ProofTest {
    let name = "Test 10 Leaves Index 4".to_string();
    let function_name = function_name!().to_string();
    let mut rng: Pcg64 = Seeder::from(&function_name).make_rng();
    let description = "\
        Build a proof from a binary Merkle tree consisting of 10 leaves and leaf index 4. \
        This proof is valid and verification is expected to pass."
        .to_string();
    let samples = 10;
    let sample_data = test_data.iter().cloned().choose_multiple(&mut rng, samples);
    let proof_index = 4;
    generate_test(name, function_name, description, &sample_data, proof_index)
}

#[named]
fn generate_test_1_leaf_index_0(test_data: &[Bytes32]) -> ProofTest {
    let name = "Test 1 Leaf Index 0".to_string();
    let function_name = function_name!().to_string();
    let mut rng: Pcg64 = Seeder::from(&function_name).make_rng();
    let description = "\
        Build a proof from a binary Merkle tree consisting of 1 leaf and leaf index 0. \
        This proof is valid and verification is expected to pass."
        .to_string();
    let samples = 1;
    let sample_data = test_data.iter().cloned().choose_multiple(&mut rng, samples);
    let proof_index = 0;
    generate_test(name, function_name, description, &sample_data, proof_index)
}

#[named]
fn generate_test_100_leaves_index_10(test_data: &[Bytes32]) -> ProofTest {
    let name = "Test 100 Leaves Index 10".to_string();
    let function_name = function_name!().to_string();
    let mut rng: Pcg64 = Seeder::from(&function_name).make_rng();
    let description = "\
        Build a proof from a binary Merkle tree consisting of 100 leaves and leaf index 10. \
        This proof is valid and verification is expected to pass."
        .to_string();
    let samples = 100;
    let sample_data = test_data.iter().cloned().choose_multiple(&mut rng, samples);
    let proof_index = 10;
    generate_test(name, function_name, description, &sample_data, proof_index)
}

#[named]
fn generate_test_1024_leaves_index_512(test_data: &[Bytes32]) -> ProofTest {
    let name = "Test 1024 Leaves Index 512".to_string();
    let function_name = function_name!().to_string();
    let mut rng: Pcg64 = Seeder::from(&function_name).make_rng();
    let description = "\
        Build a proof from a binary Merkle tree consisting of 1024 leaves and leaf index 512. \
        This proof is valid and verification is expected to pass."
        .to_string();
    let samples = 1024;
    let sample_data = test_data.iter().cloned().choose_multiple(&mut rng, samples);
    let proof_index = 512;
    generate_test(name, function_name, description, &sample_data, proof_index)
}

#[named]
fn generate_test_0_leaves(test_data: &[Bytes32]) -> ProofTest {
    let name = "Test 0 Leaves".to_string();
    let function_name = function_name!().to_string();
    let mut rng: Pcg64 = Seeder::from(&function_name).make_rng();
    let description = "\
        Build a proof from a binary Merkle tree and manual set the number of leaves to 0. \
        Setting the number of leaves to 0 implies that the source tree is empty. \
        This proof is invalid because empty trees cannot produce a proof. \
        Verification is expected to fail."
        .to_string();
    let samples = 1;
    let sample_data = test_data.iter().cloned().choose_multiple(&mut rng, samples);
    let proof_index = 0;
    let mut test =
        generate_test(name, function_name, description, &sample_data, proof_index);
    test.num_leaves = 0;
    test.expected_verification = false;
    test
}

#[named]
fn generate_test_1_leaf_invalid_proof_index(test_data: &[Bytes32]) -> ProofTest {
    let name = "Test 1 Leaf Invalid Proof Index".to_string();
    let function_name = function_name!().to_string();
    let mut rng: Pcg64 = Seeder::from(&function_name).make_rng();
    let description = "\
        Build a proof from a binary Merkle tree consisting of 1 leaf and manually set the leaf index to 1. \
        Because the leaf index is zero-based, leaf index 1 refers to a position outside the range of the source tree. \
        This proof is invalid because the leaf index is out of range. \
        Verification is expected to fail."
        .to_string();
    let samples = 1;
    let sample_data = test_data.iter().cloned().choose_multiple(&mut rng, samples);
    let proof_index = 0;
    let mut test =
        generate_test(name, function_name, description, &sample_data, proof_index);
    test.proof_index = 1;
    test.expected_verification = false;
    test
}

#[named]
fn generate_test_1_leaf_invalid_root(test_data: &[Bytes32]) -> ProofTest {
    let name = "Test 1 Leaf Invalid Root".to_string();
    let function_name = function_name!().to_string();
    let mut rng: Pcg64 = Seeder::from(&function_name).make_rng();
    let description = "\
        Build a proof from a binary Merkle tree consisting of 1 leaf and manually set the root. \
        The root is manually set to the SHA256 hash of the string \"invalid\". \
        This proof is invalid because root is not generated from canonical Merkle tree construction. \
        Verification is expected to fail."
        .to_string();
    let samples = 1;
    let sample_data = test_data.iter().cloned().choose_multiple(&mut rng, samples);
    let proof_index = 0;
    let mut test =
        generate_test(name, function_name, description, &sample_data, proof_index);
    test.root = EncodedValue::new(hex::encode(sum(b"invalid")), Encoding::Hex);
    test.expected_verification = false;
    test
}

#[named]
fn generate_test_1024_leaves_invalid_root(test_data: &[Bytes32]) -> ProofTest {
    let name = "Test 1024 Leaves Invalid Root".to_string();
    let function_name = function_name!().to_string();
    let mut rng: Pcg64 = Seeder::from(&function_name).make_rng();
    let description = "\
        Build a proof from a binary Merkle tree consisting of 1024 leaves and manually set the root. \
        The root is manually set to the SHA256 hash of the string \"invalid\". \
        This proof is invalid because root is not generated from canonical Merkle tree construction. \
        Verification is expected to fail."
        .to_string();
    let samples = 1024;
    let sample_data = test_data.iter().cloned().choose_multiple(&mut rng, samples);
    let index = 512;
    let mut test = generate_test(name, function_name, description, &sample_data, index);
    test.root = EncodedValue::new(hex::encode(sum(b"invalid")), Encoding::Hex);
    test.expected_verification = false;
    test
}

fn main() {
    let test_data_count = 2u64.pow(16);
    let test_data = (0..test_data_count)
        .map(|i| sum(&i.to_be_bytes()))
        .collect::<Vec<Bytes32>>();

    let test = generate_test_10_leaves_index_4(&test_data);
    write_test(&test);

    let test = generate_test_1_leaf_index_0(&test_data);
    write_test(&test);

    let test = generate_test_100_leaves_index_10(&test_data);
    write_test(&test);

    let test = generate_test_1024_leaves_index_512(&test_data);
    write_test(&test);

    let test = generate_test_0_leaves(&test_data);
    write_test(&test);

    let test = generate_test_1_leaf_invalid_proof_index(&test_data);
    write_test(&test);

    let test = generate_test_1_leaf_invalid_root(&test_data);
    write_test(&test);

    let test = generate_test_1024_leaves_invalid_root(&test_data);
    write_test(&test);
}

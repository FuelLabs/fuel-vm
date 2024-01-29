use crate::{
    common::{
        Bytes32,
        StorageMap,
    },
    sparse::{
        empty_sum,
        verify::verify,
        zero_sum,
        MerkleTree,
        MerkleTreeKey,
        Primitive,
    },
};
use fuel_storage::Mappable;
use proptest::{
    prop_assert,
    prop_compose,
    proptest,
};

#[derive(Debug)]
struct TestTable;

impl Mappable for TestTable {
    type Key = Self::OwnedKey;
    type OwnedKey = Bytes32;
    type OwnedValue = Primitive;
    type Value = Self::OwnedValue;
}

prop_compose! {
    fn random_tree()(key_values: Vec<(MerkleTreeKey, Bytes32)>) -> (Vec<(MerkleTreeKey, Bytes32)>, MerkleTree<TestTable, StorageMap<TestTable>>) {
        let storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(storage);
        for (key, value) in key_values.iter() {
            tree.update(*key, value).unwrap();
        }
        (key_values, tree)
    }
}

proptest! {
    #[test]
    fn generate_proof_and_verify_with_valid_key_value_returns_true((key_values, tree) in random_tree(), arb_num: usize) {
        if !key_values.is_empty() {
            let index = arb_num % key_values.len();
            let (key, value) = key_values[index];
            let proof = tree.generate_proof(key).expect("Infallible");
            let verification = verify(key, &value, proof);
            prop_assert!(verification)
        }
    }

    #[test]
    fn generate_proof_and_verify_with_valid_placeholder_returns_true((key_values, tree) in random_tree(), key: MerkleTreeKey) {
        let (keys, _values): (Vec<_>, Vec<_>) = key_values.into_iter().unzip();
        // Ensure the random key is not already included in the tree
        if !keys.iter().any(|k| *k == key) {
            let value = zero_sum();
            let proof = tree.generate_proof(key).expect("Infallible");
            let verification = verify(key, value, proof.clone());
            prop_assert!(verification)
        }
    }

    #[test]
    fn generate_proof_and_verify_with_valid_key_invalid_value_returns_false((key_values, tree) in random_tree(), arb_num: usize, value: Bytes32) {
        if !key_values.is_empty() {
            let index = arb_num % key_values.len();
            let (key, _) = key_values[index];
            let proof = tree.generate_proof(key).expect("Infallible");
            let verification = verify(key, &value, proof);
            prop_assert!(!verification)
        }
    }

    #[test]
    fn generate_proof_and_verify_with_invalid_key_value_returns_false((_, tree) in random_tree(), key: MerkleTreeKey, value: Bytes32) {
        let proof = tree.generate_proof(key).expect("Infallible");
        let verification = verify(key, &value, proof);
        prop_assert!(!verification)
    }
}

proptest! {
    #[test]
    fn verify_excluded_key_cannot_create_false_positive((key_values, tree) in random_tree(), arb_num: usize, random_key: MerkleTreeKey) {
        if !key_values.is_empty() {
            let index = arb_num % key_values.len();
            let (key, value) = key_values[index];
            let proof = tree.generate_proof(key).expect("Infallible");

            // Ensure we are testing an inclusion proof
            prop_assert!(proof.initial_hash.is_none());
            let verification = verify(key.clone(), &value, proof.clone());
            prop_assert!(verification);

            // Verify a random key using the proof. Because the random key is
            // not part of the tree, verification should fail.
            let verification = verify(random_key, empty_sum(), proof.clone());
            prop_assert!(!verification);
        }
    }

    #[test]
    fn verify_included_key_cannot_create_false_positive((_, tree) in random_tree(), random_key: MerkleTreeKey, value: Bytes32) {
        if value != *empty_sum() {
            let mut proof = tree.generate_proof(random_key).expect("Infallible");

            // Verify that the key corresponds to the zero sum. Because the key
            // is not included in the tree, verification should succeed.
            let verification = verify(random_key.clone(), empty_sum(), proof.clone());
            prop_assert!(verification);

            // Convert the exclusion proof to an inclusion proof and verify that
            // the key corresponds to the zero sum.
            proof.initial_hash = None;
            // Because the proof does not contain the correct initial hash,
            // verification should fail.
            let verification = verify(random_key.clone(), empty_sum(), proof.clone());
            prop_assert!(!verification);
        }
    }
}

use crate::{
    common::{
        Bytes32,
        StorageMap,
    },
    sparse::{
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
use rand::{
    prelude::SliceRandom,
    rngs::StdRng,
    SeedableRng,
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
    fn generate_proof_and_verify_with_valid_key_value_returns_true((key_values, tree) in random_tree()) {
        let mut rng = StdRng::seed_from_u64(0xBAADF00D);
        if let Some((key, value)) = key_values.choose(&mut rng).cloned() {
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
    fn generate_proof_and_verify_with_invalid_key_value_returns_false((_, tree) in random_tree(), key: Bytes32, value: Bytes32) {
        let key = MerkleTreeKey::new(key);
        let proof = tree.generate_proof(key).expect("Infallible");
        let verification = verify(key, &value, proof);
        prop_assert!(!verification)
    }
}

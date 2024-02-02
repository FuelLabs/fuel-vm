use crate::{
    common::{
        Bytes32,
        StorageMap,
    },
    sparse::{
        proof::Proof,
        MerkleTree,
        MerkleTreeKey,
        Primitive,
    },
};
use core::fmt::{
    Debug,
    Formatter,
};
use fuel_storage::Mappable;
use proptest::{
    arbitrary::any,
    collection::{
        hash_set,
        vec,
    },
    prelude::*,
    prop_assert,
    prop_assume,
    prop_compose,
    proptest,
    strategy::Strategy,
};
use std::collections::HashSet;

#[derive(Debug)]
struct TestTable;

impl Mappable for TestTable {
    type Key = Self::OwnedKey;
    type OwnedKey = Bytes32;
    type OwnedValue = Primitive;
    type Value = Self::OwnedValue;
}

#[derive(Copy, Clone, Eq, PartialEq, proptest_derive::Arbitrary)]
struct Value(Bytes32);

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&format!("Value({})", hex::encode(self.0)))
    }
}

impl AsRef<[u8]> for Value {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

fn keys(n: usize) -> impl Strategy<Value = HashSet<MerkleTreeKey>> {
    hash_set(any::<MerkleTreeKey>(), n)
}

fn values(n: usize) -> impl Strategy<Value = Vec<Value>> {
    vec(any::<Value>(), n)
}

prop_compose! {
    fn key_values(min: usize, max: usize)(n in min..max)(k in keys(n), v in values(n)) -> Vec<(MerkleTreeKey, Value)> {
        k.into_iter().zip(v.into_iter()).collect::<Vec<_>>()
    }
}

prop_compose! {
    fn random_tree(min: usize, max: usize)(kv in key_values(min, max)) -> (Vec<(MerkleTreeKey, Value)>, MerkleTree<TestTable, StorageMap<TestTable>>) {
        let storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(storage);
        for (key, value) in kv.iter() {
            tree.update(*key, value.as_ref()).unwrap();
        }
        (kv, tree)
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1))]

    #[test]
    fn generate_inclusion_proof_and_verify_with_valid_key_value_returns_true((key_values, tree) in random_tree(1, 100), arb_num: usize) {
        let index = arb_num % key_values.len();
        let (key, value) = key_values[index];
        let proof = tree.generate_proof(key).expect("Infallible");
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(key, &value),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        prop_assert!(inclusion)
    }

    #[test]
    fn generate_inclusion_proof_and_verify_with_valid_key_invalid_value_returns_false((key_values, tree) in random_tree(1, 100), arb_num: usize, value: Bytes32) {
        let index = arb_num % key_values.len();
        let (key, _) = key_values[index];
        let proof = tree.generate_proof(key).expect("Infallible");
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(key, &value),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        prop_assert!(!inclusion)
    }

    #[test]
    fn generate_exclusion_proof_and_verify_with_excluded_key_returns_true((key_values, tree) in random_tree(2, 100), key: MerkleTreeKey) {
        prop_assume!(!key_values.iter().any(|(k, _)| *k == key));
        dbg!(&key_values);
        dbg!(&key);
        let proof = tree.generate_proof(key).expect("Infallible");
        let root = *proof.root();
        let exclusion = match proof {
            Proof::Inclusion(_) => panic!("Expected ExclusionProof"),
            Proof::Exclusion(proof) => proof.verify(key),
        };
        println!("root: {}, exclusion: {}", hex::encode(root), exclusion);
        prop_assert!(exclusion)
    }
}

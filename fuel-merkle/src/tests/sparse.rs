#![allow(non_snake_case, clippy::arithmetic_side_effects)]

use crate::{
    common::{
        Bytes32,
        StorageMap,
    },
    sparse::{
        MerkleTree,
        MerkleTreeKey,
        Primitive,
        proof::{
            ExclusionLeaf,
            ExclusionLeafData,
            ExclusionProof,
            Proof,
        },
    },
};

use fuel_storage::Mappable;

use core::fmt::{
    Debug,
    Formatter,
};
use proptest::{
    arbitrary::any,
    collection::{
        hash_set,
        vec,
    },
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

#[derive(Copy, Clone, Eq, Hash, PartialEq, proptest_derive::Arbitrary)]
struct Key(Bytes32);

impl Debug for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&format!("Key({})", hex::encode(self.0)))
    }
}

impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Key> for Bytes32 {
    fn from(value: Key) -> Self {
        value.0
    }
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

impl From<Value> for Bytes32 {
    fn from(value: Value) -> Self {
        value.0
    }
}

fn keys(n: usize) -> impl Strategy<Value = HashSet<Key>> {
    hash_set(any::<Key>(), n)
}

fn values(n: usize) -> impl Strategy<Value = Vec<Value>> {
    vec(any::<Value>(), n)
}

prop_compose! {
    fn key_values(min: usize, max: usize)(n in min..max)(k in keys(n), v in values(n)) -> Vec<(Key, Value)> {
        k.into_iter().zip(v.into_iter()).collect::<Vec<_>>()
    }
}

prop_compose! {
    fn random_tree(min: usize, max: usize)(kv in key_values(min, max)) -> (Vec<(Key, Value)>, MerkleTree<TestTable, StorageMap<TestTable>>) {
        let storage = StorageMap::<TestTable>::new();
        let iter = kv.clone().into_iter().map(|(key, value)| (MerkleTreeKey::new(key), value));
        let tree = MerkleTree::from_set(storage, iter).expect("Unable to create Merkle tree");
        (kv, tree)
    }
}

proptest! {
    #[test]
    fn inclusion_proof__verify__returns_true_with_correct_key_and_correct_value((key_values, tree) in random_tree(1, 100), arb_num: usize) {
        let root = tree.root();

        // Given
        let index = arb_num % key_values.len();
        let (key, value) = key_values[index];
        let key = MerkleTreeKey::new(key);
        let proof = tree.generate_proof(&key).expect("Infallible");

        // When
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(&root, &key, value.as_ref()),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };

        // Then
        prop_assert!(inclusion)
    }

    #[test]
    fn inclusion_proof__verify__returns_false_with_correct_key_and_incorrect_value((key_values, tree) in random_tree(1, 100), arb_num: usize, value: Value) {
        let root = tree.root();

        // Given
        let index = arb_num % key_values.len();
        let (key, _) = key_values[index];
        let key = MerkleTreeKey::new(key);
        let proof = tree.generate_proof(&key).expect("Infallible");

        // When
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(&root, &key, value.as_ref()),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };

        // Then
        prop_assert!(!inclusion)
    }

    #[test]
    fn exclusion_proof__verify__returns_true_with_excluded_key((key_values, tree) in random_tree(1, 100), key: Key) {
        let root = tree.root();

        // Given
        prop_assume!(!key_values.iter().any(|(k, _)| *k == key));
        let key = MerkleTreeKey::new(key);
        let proof = tree.generate_proof(&key).expect("Infallible");

        // When
        let exclusion = match proof {
            Proof::Inclusion(_) => panic!("Expected ExclusionProof"),
            Proof::Exclusion(proof) => proof.verify(&root, &key),
        };

        // Then
        prop_assert!(exclusion)
    }

    #[test]
    fn exclusion_proof__verify__returns_false_for_included_key((key_values, tree) in random_tree(1, 100)) {
        let root = tree.root();

        // Given
        let (included_key, included_value) = key_values[0];
        let included_key = MerkleTreeKey::new(included_key);
        let Proof::Inclusion(inclusion_proof) = tree.generate_proof(&included_key).expect("Infallible")  else { panic!("Expected InclusionProof") };
        let exlucion_proof = ExclusionProof {
            proof_set: inclusion_proof.proof_set.clone(),
            leaf: ExclusionLeaf::Leaf(ExclusionLeafData {
                leaf_key: included_key.into(),
                leaf_value: included_value.into(),
            })
        };

        // When
        let inclusion_result = inclusion_proof.verify(&root, &included_key, included_value.as_ref());
        let exclusion_result = exlucion_proof.verify(&root, &included_key);

        // Then
        prop_assert!(inclusion_result);
        prop_assert!(!exclusion_result);
        prop_assert!(inclusion_result != exclusion_result);
    }
}

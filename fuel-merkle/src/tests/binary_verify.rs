#![allow(non_snake_case, clippy::arithmetic_side_effects)]

use core::fmt::{
    Debug,
    Formatter,
};

use proptest::{
    arbitrary::any,
    collection::vec,
    prop_assert,
    prop_compose,
    proptest,
    strategy::Strategy,
};

use crate::{
    binary::{
        verify,
        MerkleTree,
        Primitive,
    },
    common::{
        Bytes32,
        ProofSet,
        StorageMap,
    },
};
use fuel_storage::Mappable;

#[derive(Debug)]
struct TestTable;

impl Mappable for TestTable {
    type Key = Self::OwnedKey;
    type OwnedKey = u64;
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

impl From<Value> for Bytes32 {
    fn from(value: Value) -> Self {
        value.0
    }
}

fn _values(n: usize) -> impl Strategy<Value = Vec<Value>> {
    vec(any::<Value>(), n)
}

prop_compose! {
    fn values(min: usize, max: usize)(n in min..max)(v in _values(n)) -> Vec<Value> {
        v.into_iter().collect::<Vec<_>>()
    }
}

prop_compose! {
    fn random_tree(min: usize, max: usize)(values in values(min, max)) -> (Vec<Value>, MerkleTree<TestTable, StorageMap<TestTable>>) {
        let storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(storage);
        for datum in values.iter() {
            tree.push(datum.as_ref()).unwrap();
        }
        (values, tree)
    }
}

proptest! {
    #[test]
    fn verify__returns_true_for_valid_proof((values, tree) in random_tree(1, 1_000), arb_num: usize){
        let num_leaves = values.len();
        let index = arb_num % num_leaves;
        let data = values[index];

        // Given
        let (root, proof_set) = tree.prove(index  as u64).expect("Unable to generate proof");

        // When
        let verification = verify(&root, &data, &proof_set, index as u64, num_leaves  as u64);

        // Then
        prop_assert!(verification)
    }

    #[test]
    fn verify__returns_false_for_invalid_root((values, tree) in random_tree(1, 1_000), arb_num: usize, root: Bytes32){
        let num_leaves = values.len();
        let index = arb_num % num_leaves;
        let data = values[index];

        // Given
        let (_, proof_set) = tree.prove(index  as u64).expect("Unable to generate proof");

        // When
        let verification = verify(&root, &data, &proof_set, index as u64, num_leaves  as u64);

        // Then
        prop_assert!(!verification)
    }

    #[test]
    fn verify__returns_false_for_invalid_proof_set((values, tree) in random_tree(2, 1_000), arb_num: usize, proof_set: ProofSet){
        let num_leaves = values.len();
        let index = arb_num % num_leaves;
        let data = values[index];

        // Given
        let (root, _) = tree.prove(index  as u64).expect("Unable to generate proof");

        // When
        let verification = verify(&root, &data, &proof_set, index as u64, num_leaves  as u64);

        // Then
        prop_assert!(!verification)
    }

    #[test]
    fn verify__returns_true_for_valid_proof_of_last_leaf((values, tree) in random_tree(1, 1_000)){
        let num_leaves = values.len();
        let index = num_leaves - 1;
        let data = values[index];

        // Given
        let (root, proof_set) = tree.prove(index as u64).expect("Unable to generate proof");

        // When
        let verification = verify(&root, &data, &proof_set, index as u64, num_leaves as u64);

        // Then
        prop_assert!(verification)
    }

    #[test]
    fn verify__returns_false_for_invalid_proof_of_last_leaf((values, tree) in random_tree(1, 1_000), incorrect_num_leaves: u64){
        let num_leaves = values.len();
        proptest::prop_assume!(num_leaves as u64 != incorrect_num_leaves);

        let index = num_leaves - 1;
        let data = values[index];

        // Given
        let (root, proof_set) = tree.prove(index as u64).expect("Unable to generate proof");

        // When
        let verification = verify(&root, &data, &proof_set, index as u64, incorrect_num_leaves);

        // Then
        prop_assert!(!verification)
    }

    #[test]
    fn verify__returns_false_for_invalid_proof_index((values, tree) in random_tree(1, 1_000), invalid_index: u64){
        let num_leaves = values.len();
        let valid_index = num_leaves - 1;
        proptest::prop_assume!(invalid_index != valid_index as u64);
        let data = values[valid_index];

        // Given
        let (root, proof_set) = tree.prove(valid_index as u64).expect("Unable to generate proof");

        // When
        let verification = verify(&root, &data, &proof_set, invalid_index, num_leaves as u64);

        // Then
        prop_assert!(!verification)
    }

    #[test]
    fn verify__returns_false_for_invalid_proof_index_and_num_leaves((values, tree) in random_tree(1, 1_000), invalid_index: u64, incorrect_num_leaves: u64){
        let num_leaves = values.len();
        let valid_index = num_leaves - 1;
        proptest::prop_assume!(invalid_index != valid_index as u64);
        let data = values[valid_index];

        // Given
        let (root, proof_set) = tree.prove(valid_index as u64).expect("Unable to generate proof");

        // When
        let verification = verify(&root, &data, &proof_set, invalid_index, incorrect_num_leaves);

        // Then
        prop_assert!(!verification)
    }
}

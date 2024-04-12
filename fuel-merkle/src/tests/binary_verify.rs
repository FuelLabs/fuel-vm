#![allow(non_snake_case)]

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
    fn verify__returns_true_for_valid_proof((values, tree) in random_tree(1, 1_000), arb_num: u64){
        let num_leaves = values.len() as u64;
        let index = arb_num % num_leaves;
        let data = values[index as usize];

        // Given
        let (root, proof_set) = tree.prove(index).expect("Unable to generate proof");

        // When
        let verification = verify(&root, &data, &proof_set, index, num_leaves);

        // Then
        prop_assert!(verification)
    }

    #[test]
    fn verify__returns_true_for_valid_proof_of_last_leaf((values, tree) in random_tree(1, 1_000)){
        let num_leaves = values.len() as u64;
        let index = num_leaves - 1;
        let data = values[index as usize];

        // Given
        let (root, proof_set) = tree.prove(index).expect("Unable to generate proof");

        // When
        let verification = verify(&root, &data, &proof_set, index, num_leaves);

        // Then
        prop_assert!(verification)
    }

    #[test]
    fn verify__returns_false_for_invalid_proof_of_last_leaf((values, tree) in random_tree(1, 1_000)){
        let num_leaves = values.len() as u64;
        let index = num_leaves - 1;
        let data = values[index as usize];

        // Given
        let (root, proof_set) = tree.prove(index).expect("Unable to generate proof");

        // When
        let verification = verify(&root, &data, &proof_set, index, num_leaves + 1);

        // Then
        prop_assert!(!verification)
    }
}

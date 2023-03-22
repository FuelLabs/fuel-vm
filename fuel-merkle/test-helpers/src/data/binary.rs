use fuel_merkle::binary::verify;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

use fuel_merkle::common::Bytes32;

use crate::{
    binary::verify as verify_from_test_helper,
    data::{EncodedValue, TestError},
};

#[derive(Serialize, Deserialize)]
pub struct ProofTest {
    pub name: String,
    pub function_name: String,
    pub description: String,
    pub root: EncodedValue,
    pub data: Bytes32,
    pub proof_set: Vec<EncodedValue>,
    pub proof_index: u64,
    pub num_leaves: u64,
    pub expected_verification: bool,
}

impl ProofTest {
    pub fn execute(self) -> Result<(), TestError> {
        let root: Bytes32 = self.root.into_bytes()?.as_slice().try_into().unwrap();
        let proof_set = self
            .proof_set
            .iter()
            .cloned()
            .map(|v| v.into_bytes().unwrap().as_slice().try_into().unwrap())
            .collect::<Vec<Bytes32>>();

        let verification = verify(&root, self.data, &proof_set, self.proof_index, self.num_leaves);
        let verification_from_test_helper =
            verify_from_test_helper(&root, &proof_set, self.proof_index, self.num_leaves);
        assert_eq!(verification, verification_from_test_helper);

        if verification == self.expected_verification {
            Ok(())
        } else {
            Err(TestError::Failed(self.name))
        }
    }
}

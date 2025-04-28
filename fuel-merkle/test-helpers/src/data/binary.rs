use fuel_merkle::binary::verify;
use serde::{
    Deserialize,
    Serialize,
};
use std::convert::TryInto;

use fuel_merkle::common::Bytes32;

use crate::{
    binary::verify as verify_from_test_helper,
    data::{
        EncodedValue,
        TestError,
    },
};

#[derive(Serialize, Deserialize)]
pub struct ProofTest {
    pub name: String,
    pub function_name: String,
    pub description: String,
    pub root: EncodedValue,
    pub data: EncodedValue,
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
        let data = self.data.into_bytes()?;
        let verification =
            verify(&root, &data, &proof_set, self.proof_index, self.num_leaves);
        let verification_from_test_helper = verify_from_test_helper(
            &root,
            &data,
            &proof_set,
            self.proof_index,
            self.num_leaves,
        );
        let expected_verification = self.expected_verification;

        if verification != verification_from_test_helper {
            return Err(TestError::Failed(
                self.name,
                format!(
                    "Verification {verification} does not match reference verification {verification_from_test_helper}",
                ),
            ));
        }

        if verification != expected_verification {
            return Err(TestError::Failed(
                self.name,
                format!(
                    "Verification {verification} does not match expected verification {expected_verification}",
                ),
            ));
        }

        Ok(())
    }
}

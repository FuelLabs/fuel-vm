use fuel_merkle::{
    common::Bytes32,
    sparse::{
        MerkleTreeKey,
        in_memory,
    },
};
use serde::Deserialize;
use std::convert::TryInto;

use crate::data::{
    EncodedValue,
    TestError,
};

// Supported actions:
const ACTION_UPDATE: &str = "update";
const ACTION_DELETE: &str = "delete";

trait MerkleTreeTestAdaptor {
    fn update(&mut self, key: &Bytes32, data: &[u8]);
    fn delete(&mut self, key: &Bytes32);
    fn root(&self) -> Bytes32;
}

#[derive(Deserialize)]
struct Step {
    action: String,
    key: Option<EncodedValue>,
    data: Option<EncodedValue>,
}

enum Action {
    Update(EncodedValue, EncodedValue),
    Delete(EncodedValue),
}

impl Step {
    pub fn execute(self, tree: &mut dyn MerkleTreeTestAdaptor) -> Result<(), TestError> {
        match self.action_type()? {
            Action::Update(encoded_key, encoded_data) => {
                let key_bytes = encoded_key.into_bytes()?;
                let key = &key_bytes.try_into().unwrap();
                let data_bytes = encoded_data.into_bytes()?;
                let data: &[u8] = &data_bytes;
                tree.update(key, data);
                Ok(())
            }
            Action::Delete(encoded_key) => {
                let key_bytes = encoded_key.into_bytes()?;
                let key = &key_bytes.try_into().unwrap();
                tree.delete(key);
                Ok(())
            }
        }
    }

    // Translate the action string found in the step definition to an Action enum variant
    // with the appropriate key and data bindings.
    fn action_type(&self) -> Result<Action, TestError> {
        match self.action.as_str() {
            // An Update has a key and data
            ACTION_UPDATE => Ok(Action::Update(
                self.key.clone().unwrap(),
                self.data.clone().unwrap(),
            )),

            // A Delete has a key
            ACTION_DELETE => Ok(Action::Delete(self.key.clone().unwrap())),

            // Unsupported action
            _ => Err(TestError::UnsupportedAction(self.action.clone())),
        }
    }
}

struct InMemoryMerkleTreeTestAdaptor {
    tree: Box<in_memory::MerkleTree>,
}

impl MerkleTreeTestAdaptor for InMemoryMerkleTreeTestAdaptor {
    fn update(&mut self, key: &Bytes32, data: &[u8]) {
        self.tree
            .as_mut()
            .update(MerkleTreeKey::new_without_hash(*key), data)
    }

    fn delete(&mut self, key: &Bytes32) {
        self.tree
            .as_mut()
            .delete(MerkleTreeKey::new_without_hash(*key))
    }

    fn root(&self) -> Bytes32 {
        self.tree.as_ref().root()
    }
}

#[derive(Deserialize)]
pub struct Test {
    name: String,
    expected_root: EncodedValue,
    steps: Vec<Step>,
}

impl Test {
    pub fn execute(self) -> Result<(), TestError> {
        let tree = Box::new(in_memory::MerkleTree::new());
        let mut tree = InMemoryMerkleTreeTestAdaptor { tree };

        for step in self.steps {
            step.execute(&mut tree)?
        }

        let root = tree.root();
        let expected_root: Bytes32 = self.expected_root.into_bytes()?.try_into().unwrap();

        if root == expected_root {
            Ok(())
        } else {
            Err(TestError::Failed(
                self.name,
                format!(
                    "Root 0x{} does not match expected root 0x{}",
                    hex::encode(root),
                    hex::encode(expected_root)
                ),
            ))
        }
    }
}

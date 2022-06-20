use fuel_merkle::common::Bytes32;
use fuel_merkle::sparse::in_memory;
use serde::Deserialize;
use std::convert::TryInto;
use std::fmt::{Display, Formatter};
use std::{fs::File, path::Path};
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum TestError {
    #[error("Test failed")]
    Failed,
    #[error("Unsupported action {0}")]
    UnsupportedAction(String),
    #[error("Unsupported encoding {0}")]
    UnsupportedEncoding(String),
}

// Supported actions:
const ACTION_UPDATE: &str = "update";
const ACTION_DELETE: &str = "delete";

// Supported value encodings:
const ENCODING_HEX: &str = "hex";
const ENCODING_UTF8: &str = "utf-8";

#[derive(Deserialize, Clone)]
struct EncodedValue {
    value: String,
    encoding: String,
}

enum Encoding {
    Hex,
    Utf8,
}

impl EncodedValue {
    fn to_bytes(self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match self.encoding_type()? {
            Encoding::Hex => Ok(hex::decode(self.value).unwrap()),
            Encoding::Utf8 => Ok(self.value.into_bytes()),
        }
    }

    // Translate the encoding string found in the value definition to an Encoding enum variant.
    fn encoding_type(&self) -> Result<Encoding, Box<dyn std::error::Error>> {
        match self.encoding.as_str() {
            ENCODING_HEX => Ok(Encoding::Hex),
            ENCODING_UTF8 => Ok(Encoding::Utf8),

            // Unsupported encoding
            _ => Err(Box::<TestError>::new(TestError::UnsupportedEncoding(
                self.encoding.clone(),
            ))),
        }
    }
}

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
    pub fn execute(
        self,
        tree: &mut dyn MerkleTreeTestAdaptor,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self.action_type()? {
            Action::Update(encoded_key, encoded_data) => {
                let key_bytes = encoded_key.to_bytes()?;
                let key = &key_bytes.try_into().unwrap();
                let data_bytes = encoded_data.to_bytes()?;
                let data: &[u8] = &data_bytes;
                tree.update(key, data);
                Ok(())
            }
            Action::Delete(encoded_key) => {
                let key_bytes = encoded_key.to_bytes()?;
                let key = &key_bytes.try_into().unwrap();
                tree.delete(key);
                Ok(())
            }
        }
    }

    // Translate the action string found in the step definition to an Action enum variant with the
    // appropriate key and data bindings.
    fn action_type(&self) -> Result<Action, Box<dyn std::error::Error>> {
        match self.action.as_str() {
            // An Update has a key and data
            ACTION_UPDATE => Ok(Action::Update(
                self.key.clone().unwrap(),
                self.data.clone().unwrap(),
            )),

            // A Delete has a key
            ACTION_DELETE => Ok(Action::Delete(self.key.clone().unwrap())),

            // Unsupported action
            _ => Err(Box::<TestError>::new(TestError::UnsupportedAction(
                self.action.clone(),
            ))),
        }
    }
}

struct InMemoryMerkleTreeTestAdaptor {
    tree: Box<in_memory::MerkleTree>,
}

impl<'a> MerkleTreeTestAdaptor for InMemoryMerkleTreeTestAdaptor {
    fn update(&mut self, key: &Bytes32, data: &[u8]) {
        self.tree.as_mut().update(key, data)
    }

    fn delete(&mut self, key: &Bytes32) {
        self.tree.as_mut().delete(key)
    }

    fn root(&self) -> Bytes32 {
        self.tree.as_ref().root()
    }
}

#[derive(Deserialize)]
struct Test {
    name: String,
    expected_root: EncodedValue,
    steps: Vec<Step>,
}

impl Test {
    pub fn execute(self) -> Result<(), Box<dyn std::error::Error>> {
        let tree = Box::new(in_memory::MerkleTree::new());
        let mut tree = InMemoryMerkleTreeTestAdaptor { tree };

        for step in self.steps {
            step.execute(&mut tree)?
        }

        let root = tree.root();
        let expected_root: Bytes32 = self.expected_root.to_bytes()?.try_into().unwrap();

        assert_eq!(root, expected_root);

        Ok(())
    }
}

impl Display for Test {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

fn test(path: &Path) -> datatest_stable::Result<()> {
    let data_file = File::open(path)?;
    let test: Test = serde_yaml::from_reader(data_file)?;
    test.execute()
}

datatest_stable::harness!(test, "./tests-data/fixtures", r"^.*/*");

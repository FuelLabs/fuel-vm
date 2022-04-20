use std::{fs::File, path::Path};

use fuel_merkle::common::{Bytes32, StorageError, StorageMap};
use fuel_merkle::sparse::MerkleTree;
use serde::Deserialize;
use std::convert::TryInto;
use std::fmt::{Display, Formatter};

use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum TestError {
    #[error("Test failed")]
    Failed,
    #[error("Unsupported action `{0}")]
    UnsupportedAction(String),
    #[error("Unsupported encoding `{0}")]
    UnsupportedEncoding(String),
}

const BUFFER_SIZE: usize = 69;
pub type Buffer = [u8; BUFFER_SIZE];

#[derive(Deserialize)]
struct EncodedValue {
    value: String,
    encoding: String,
}

impl EncodedValue {
    fn to_bytes(self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match self.encoding.as_str() {
            "hex" => Ok(hex::decode(self.value).unwrap()),
            "utf-8" => Ok(self.value.into_bytes()),

            // Unsupported encoding
            _ => Err(Box::<TestError>::new(TestError::UnsupportedEncoding(
                self.encoding,
            ))),
        }
    }
}

#[derive(Deserialize)]
struct Step {
    action: String,
    key: Option<EncodedValue>,
    data: Option<EncodedValue>,
}

impl Step {
    pub fn execute(
        self,
        tree: &mut MerkleTree<StorageError>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self.action.as_str() {
            "update" => {
                let key_bytes = self.key.unwrap().to_bytes()?;
                let key = &key_bytes.try_into().unwrap();
                let data_bytes = self.data.unwrap().to_bytes()?;
                let data: &[u8] = &data_bytes;
                tree.update(key, data).unwrap();
                Ok(())
            }
            "delete" => {
                let key_bytes = self.key.unwrap().to_bytes()?;
                let key = &key_bytes.try_into().unwrap();
                tree.delete(key).unwrap();
                Ok(())
            }

            // Unsupported action
            _ => Err(Box::<TestError>::new(TestError::UnsupportedAction(
                self.action,
            ))),
        }
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
        let mut storage = StorageMap::<Bytes32, Buffer>::new();
        let mut tree = MerkleTree::<StorageError>::new(&mut storage);

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

use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("generic error occurred")]
    Error(Box<dyn std::error::Error + Send>),
}

pub trait Storage<Key, Value> {
    fn insert(&mut self, key: &Key, value: &Value) -> Result<Option<Value>, StorageError>;

    fn remove(&mut self, key: &Key) -> Result<Option<Value>, StorageError>;

    fn get(&self, key: &Key) -> Result<Option<Value>, StorageError>;

    fn contains_key(&self, key: &Key) -> Result<bool, StorageError>;
}

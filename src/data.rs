use crate::interpreter::Contract;

use fuel_asm::Word;
use fuel_tx::{Color, ContractId};

use std::ops::DerefMut;

mod error;
mod memory;

pub use error::DataError;
pub use memory::MemoryStorage;

pub trait Key {}

pub trait Value {}

pub trait Storage<K, V>
where
    K: Key,
    V: Value,
{
    fn insert(&mut self, key: K, value: V) -> Result<Option<V>, DataError>;
    fn remove(&mut self, key: &K) -> Result<Option<V>, DataError>;

    // This initial implementation safeguard from the complex scenarios when a
    // reference is returned. To simplify, at least for now, we return the owned
    // value.
    fn get(&self, key: &K) -> Result<Option<V>, DataError>;
    fn contains_key(&self, key: &K) -> Result<bool, DataError>;
}

impl<K, V, S, I> Storage<K, V> for I
where
    K: Key,
    V: Value,
    S: Storage<K, V>,
    I: DerefMut<Target = S>,
{
    fn insert(&mut self, key: K, value: V) -> Result<Option<V>, DataError> {
        <S as Storage<K, V>>::insert(self.deref_mut(), key, value)
    }

    fn remove(&mut self, key: &K) -> Result<Option<V>, DataError> {
        <S as Storage<K, V>>::remove(self.deref_mut(), key)
    }

    fn get(&self, key: &K) -> Result<Option<V>, DataError> {
        <S as Storage<K, V>>::get(self.deref(), key)
    }

    fn contains_key(&self, key: &K) -> Result<bool, DataError> {
        <S as Storage<K, V>>::contains_key(self.deref(), key)
    }
}

/// When this trait is implemented, the underlying interpreter is guaranteed to
/// have full functionality
pub trait InterpreterStorage: Storage<ContractId, Contract> + Storage<(ContractId, Color), Word> {}
impl<S, I> InterpreterStorage for I
where
    S: InterpreterStorage,
    I: DerefMut<Target = S>,
{
}

// Provisory implementation that will cover ID definitions until client backend
// is implemented
impl Key for ContractId {}
impl Key for (ContractId, Color) {}
impl Value for Word {}
impl Value for Contract {}

use crate::interpreter::{BlockData, Contract};

use fuel_asm::Word;
use fuel_tx::{Address, Bytes32, Color, ContractId};

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

pub trait MerkleStorage<K, V>: Storage<K, V>
where
    K: Key,
    V: Value,
{
    fn root(&mut self) -> Result<Bytes32, DataError>;
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
pub trait InterpreterStorage:
    Storage<ContractId, Contract>
    + Storage<(ContractId, Color), Word>
    + Storage<(ContractId, Bytes32), Bytes32>
    + MerkleStorage<Word, [u8; 8]>
{
    fn block_height(&self) -> Result<u32, DataError>;
    fn coinbase(&self) -> Result<Address, DataError>;
    fn block_data(&self, block_height: u32) -> Result<BlockData, DataError>;
}

impl<S, I> InterpreterStorage for I
where
    S: InterpreterStorage,
    I: DerefMut<Target = S> + MerkleStorage<Word, [u8; 8]>,
{
    fn block_height(&self) -> Result<u32, DataError> {
        S::block_height(self)
    }

    fn coinbase(&self) -> Result<Address, DataError> {
        S::coinbase(self)
    }

    fn block_data(&self, block_height: u32) -> Result<BlockData, DataError> {
        S::block_data(self, block_height)
    }
}

// Provisory implementation that will cover ID definitions until client backend
// is implemented
impl Key for Word {}
impl Key for ContractId {}
impl Key for (ContractId, Color) {}
impl Key for (ContractId, Bytes32) {}
impl Value for Word {}
impl Value for Contract {}
impl Value for Bytes32 {}
impl Value for [u8; 8] {}

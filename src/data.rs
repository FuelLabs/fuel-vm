use crate::interpreter::Contract;

use fuel_asm::Word;
use fuel_tx::{Address, Bytes32, Color, ContractId, Salt};

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
    fn insert(&mut self, key: &K, value: &V) -> Result<Option<V>, DataError>;
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
    fn insert(&mut self, key: &K, value: &V) -> Result<Option<V>, DataError> {
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

pub trait MerkleStorage<P, K, V>
where
    P: Key,
    K: Key,
    V: Value,
{
    fn insert(&mut self, parent: &P, key: &K, value: &V) -> Result<Option<V>, DataError>;
    fn remove(&mut self, parent: &P, key: &K) -> Result<Option<V>, DataError>;
    fn get(&self, parent: &P, key: &K) -> Result<Option<V>, DataError>;
    fn contains_key(&self, parent: &P, key: &K) -> Result<bool, DataError>;
    fn root(&mut self, parent: &P) -> Result<Bytes32, DataError>;
}

impl<P, K, V, X, I> MerkleStorage<P, K, V> for I
where
    P: Key,
    K: Key,
    V: Value,
    X: MerkleStorage<P, K, V>,
    I: DerefMut<Target = X>,
{
    fn insert(&mut self, parent: &P, key: &K, value: &V) -> Result<Option<V>, DataError> {
        <X as MerkleStorage<P, K, V>>::insert(self.deref_mut(), parent, key, value)
    }

    fn remove(&mut self, parent: &P, key: &K) -> Result<Option<V>, DataError> {
        <X as MerkleStorage<P, K, V>>::remove(self.deref_mut(), parent, key)
    }

    fn get(&self, parent: &P, key: &K) -> Result<Option<V>, DataError> {
        <X as MerkleStorage<P, K, V>>::get(self.deref(), parent, key)
    }

    fn contains_key(&self, parent: &P, key: &K) -> Result<bool, DataError> {
        <X as MerkleStorage<P, K, V>>::contains_key(self.deref(), parent, key)
    }

    fn root(&mut self, parent: &P) -> Result<Bytes32, DataError> {
        <X as MerkleStorage<P, K, V>>::root(self.deref_mut(), parent)
    }
}

// TODO use trait aliases after stable release
// https://github.com/rust-lang/rust/issues/41517

/// When this trait is implemented, the underlying interpreter is guaranteed to
/// have full functionality
pub trait InterpreterStorage:
    Storage<ContractId, (Salt, Bytes32)>
    + Storage<ContractId, Contract>
    + MerkleStorage<ContractId, Color, Word>
    + MerkleStorage<ContractId, Bytes32, Bytes32>
{
    fn block_height(&self) -> Result<u32, DataError>;
    fn block_hash(&self, block_height: u32) -> Result<Bytes32, DataError>;
    fn coinbase(&self) -> Result<Address, DataError>;
}

impl<S, I> InterpreterStorage for I
where
    S: InterpreterStorage,
    I: DerefMut<Target = S>,
{
    fn block_height(&self) -> Result<u32, DataError> {
        <S as InterpreterStorage>::block_height(self.deref())
    }

    fn block_hash(&self, block_height: u32) -> Result<Bytes32, DataError> {
        <S as InterpreterStorage>::block_hash(self.deref(), block_height)
    }

    fn coinbase(&self) -> Result<Address, DataError> {
        <S as InterpreterStorage>::coinbase(self.deref())
    }
}

// Provisory implementation that will cover ID definitions until client backend
// is implemented
impl Key for Bytes32 {}
impl Key for Color {}
impl Key for ContractId {}
impl Key for Word {}

impl Value for Bytes32 {}
impl Value for Contract {}
impl Value for Salt {}
impl Value for Word {}

impl<P, K> Key for (P, K)
where
    P: Key,
    K: Key,
{
}

impl<A, B> Value for (A, B)
where
    A: Value,
    B: Value,
{
}

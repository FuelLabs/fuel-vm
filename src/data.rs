use crate::interpreter::{BlockData, Contract, ContractData, ContractState};

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

pub trait KeyedMerkleStorage<P, M, K, V>: Storage<(P, K), V>
where
    P: Key,
    M: Value,
    K: Key,
    V: Value,
{
    fn initialize(&mut self, parent: P, metadata: M) -> Result<(), DataError>;
    fn metadata(&self, parent: &P) -> Result<M, DataError>;
    fn update(&mut self, parent: &P, metadata: M) -> Result<(), DataError>;
    fn destroy(&mut self, parent: &P) -> Result<(), DataError>;
    fn root(&mut self, parent: &P) -> Result<Bytes32, DataError>;
}

impl<P, M, K, V, X, I> KeyedMerkleStorage<P, M, K, V> for I
where
    P: Key,
    M: Value,
    K: Key,
    V: Value,
    X: Storage<(P, K), V>,
    X: KeyedMerkleStorage<P, M, K, V>,
    I: DerefMut<Target = X>,
{
    fn initialize(&mut self, parent: P, metadata: M) -> Result<(), DataError> {
        <X as KeyedMerkleStorage<P, M, K, V>>::initialize(self.deref_mut(), parent, metadata)
    }

    fn metadata(&self, parent: &P) -> Result<M, DataError> {
        <X as KeyedMerkleStorage<P, M, K, V>>::metadata(self.deref(), parent)
    }

    fn update(&mut self, parent: &P, metadata: M) -> Result<(), DataError> {
        <X as KeyedMerkleStorage<P, M, K, V>>::update(self.deref_mut(), parent, metadata)
    }

    fn destroy(&mut self, parent: &P) -> Result<(), DataError> {
        <X as KeyedMerkleStorage<P, M, K, V>>::destroy(self.deref_mut(), parent)
    }

    fn root(&mut self, parent: &P) -> Result<Bytes32, DataError> {
        <X as KeyedMerkleStorage<P, M, K, V>>::root(self.deref_mut(), parent)
    }
}

/// When this trait is implemented, the underlying interpreter is guaranteed to
/// have full functionality
pub trait InterpreterStorage:
    KeyedMerkleStorage<ContractId, ContractData, (), ContractState>
    + KeyedMerkleStorage<ContractId, (), Color, Word>
    + KeyedMerkleStorage<ContractId, (), Bytes32, Bytes32>
{
    fn block_height(&self) -> Result<u32, DataError>;
    fn coinbase(&self) -> Result<Address, DataError>;
    fn block_data(&self, block_height: u32) -> Result<BlockData, DataError>;
}

impl<S, I> InterpreterStorage for I
where
    S: InterpreterStorage,
    I: DerefMut<Target = S>,
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
impl Key for () {}
impl Key for Word {}
impl Key for ContractId {}
impl Key for Color {}
impl Key for Bytes32 {}

impl Value for () {}
impl Value for Word {}
impl Value for Contract {}
impl Value for Bytes32 {}
impl Value for ContractState {}
impl Value for ContractData {}

impl<K> Key for &K where K: Key {}
impl<V> Value for &V where V: Value {}

impl<P, K> Key for (P, K)
where
    P: Key,
    K: Key,
{
}

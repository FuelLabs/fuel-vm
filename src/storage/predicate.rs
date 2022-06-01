use std::borrow::Cow;

use crate::error::InterpreterError;
use crate::storage::InterpreterStorage;

use fuel_asm::Word;
use fuel_storage::{MerkleRoot, MerkleStorage, Storage};
use fuel_tx::Contract;
use fuel_types::{Address, AssetId, Bytes32, ContractId, Salt};

/// No-op storage used for predicate operations.
///
/// The storage implementations are expected to provide KV-like operations for contract operations.
/// However, predicates, as defined in the protocol, cannot execute contract opcodes. This means
/// its storage backend for predicate execution shouldn't provide any functionality.
#[derive(Debug, Default, Clone, Copy)]
pub struct PredicateStorage;

impl Storage<ContractId, Contract> for PredicateStorage {
    type Error = InterpreterError;

    fn insert(&mut self, _key: &ContractId, _value: &Contract) -> Result<Option<Contract>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn remove(&mut self, _key: &ContractId) -> Result<Option<Contract>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn get(&self, _key: &ContractId) -> Result<Option<Cow<'_, Contract>>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn contains_key(&self, _key: &ContractId) -> Result<bool, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }
}

impl Storage<ContractId, (Salt, Bytes32)> for PredicateStorage {
    type Error = InterpreterError;

    fn insert(
        &mut self,
        _key: &ContractId,
        _value: &(Salt, Bytes32),
    ) -> Result<Option<(Salt, Bytes32)>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn remove(&mut self, _key: &ContractId) -> Result<Option<(Salt, Bytes32)>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn get(&self, _key: &ContractId) -> Result<Option<Cow<'_, (Salt, Bytes32)>>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn contains_key(&self, _key: &ContractId) -> Result<bool, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }
}

impl MerkleStorage<ContractId, AssetId, Word> for PredicateStorage {
    type Error = InterpreterError;

    fn insert(
        &mut self,
        _parent: &ContractId,
        _key: &AssetId,
        _value: &Word,
    ) -> Result<Option<Word>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn get(&self, _parent: &ContractId, _key: &AssetId) -> Result<Option<Cow<'_, Word>>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn remove(&mut self, _parent: &ContractId, _key: &AssetId) -> Result<Option<Word>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn contains_key(&self, _parent: &ContractId, _key: &AssetId) -> Result<bool, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn root(&mut self, _parent: &ContractId) -> Result<MerkleRoot, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }
}

impl MerkleStorage<ContractId, Bytes32, Bytes32> for PredicateStorage {
    type Error = InterpreterError;

    fn insert(
        &mut self,
        _parent: &ContractId,
        _key: &Bytes32,
        _value: &Bytes32,
    ) -> Result<Option<Bytes32>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn get(&self, _parent: &ContractId, _key: &Bytes32) -> Result<Option<Cow<'_, Bytes32>>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn remove(&mut self, _parent: &ContractId, _key: &Bytes32) -> Result<Option<Bytes32>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn contains_key(&self, _parent: &ContractId, _key: &Bytes32) -> Result<bool, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn root(&mut self, _parent: &ContractId) -> Result<MerkleRoot, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }
}

impl InterpreterStorage for PredicateStorage {
    type DataError = InterpreterError;

    fn block_height(&self) -> Result<u32, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn block_hash(&self, _block_height: u32) -> Result<Bytes32, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn coinbase(&self) -> Result<Address, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }
}

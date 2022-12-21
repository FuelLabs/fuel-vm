use std::borrow::Cow;

use crate::error::InterpreterError;
use crate::storage::InterpreterStorage;

use fuel_asm::Word;
use fuel_storage::{Mappable, MerkleRoot, MerkleRootStorage, StorageInspect, StorageMutate};
use fuel_types::{Address, Bytes32, ContractId};

/// No-op storage used for predicate operations.
///
/// The storage implementations are expected to provide KV-like operations for contract operations.
/// However, predicates, as defined in the protocol, cannot execute contract opcodes. This means
/// its storage backend for predicate execution shouldn't provide any functionality.
#[derive(Debug, Default, Clone, Copy)]
pub struct PredicateStorage;

impl<Type: Mappable> StorageInspect<Type> for PredicateStorage {
    type Error = InterpreterError;

    fn get(&self, _key: &Type::Key) -> Result<Option<Cow<'_, Type::GetValue>>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn contains_key(&self, _key: &Type::Key) -> Result<bool, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }
}

impl<Type: Mappable> StorageMutate<Type> for PredicateStorage {
    fn insert(
        &mut self,
        _key: &Type::Key,
        _value: &Type::SetValue,
    ) -> Result<Option<Type::GetValue>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn remove(&mut self, _key: &Type::Key) -> Result<Option<Type::GetValue>, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }
}

impl<Key, Type: Mappable> MerkleRootStorage<Key, Type> for PredicateStorage {
    fn root(&mut self, _parent: &Key) -> Result<MerkleRoot, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }
}

impl InterpreterStorage for PredicateStorage {
    type DataError = InterpreterError;

    fn block_height(&self) -> Result<u32, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn timestamp(&self, _height: u32) -> Result<Word, Self::DataError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn block_hash(&self, _block_height: u32) -> Result<Bytes32, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn coinbase(&self) -> Result<Address, InterpreterError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn merkle_contract_state_range(
        &self,
        _id: &ContractId,
        _start_key: &Bytes32,
        _range: Word,
    ) -> Result<Vec<Option<Cow<Bytes32>>>, Self::DataError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn merkle_contract_state_insert_range(
        &mut self,
        _contract: &ContractId,
        _start_key: &Bytes32,
        _values: &[Bytes32],
    ) -> Result<Option<()>, Self::DataError> {
        Err(InterpreterError::PredicateFailure)
    }

    fn merkle_contract_state_remove_range(
        &mut self,
        _contract: &ContractId,
        _start_key: &Bytes32,
        _range: Word,
    ) -> Result<Option<()>, Self::DataError> {
        Err(InterpreterError::PredicateFailure)
    }
}

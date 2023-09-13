use alloc::{
    borrow::Cow,
    vec::Vec,
};

use crate::{
    error::InterpreterError,
    prelude::RuntimeError,
    storage::InterpreterStorage,
};

use fuel_asm::Word;
use fuel_storage::{
    Mappable,
    MerkleRoot,
    MerkleRootStorage,
    StorageInspect,
    StorageMutate,
    StorageRead,
    StorageSize,
};
use fuel_types::{
    Address,
    BlockHeight,
    Bytes32,
    ContractId,
};

use super::{
    interpreter::ContractsAssetsStorage,
    ContractsRawCode,
};

/// No-op storage used for predicate operations.
///
/// The storage implementations are expected to provide KV-like operations for contract
/// operations. However, predicates, as defined in the protocol, cannot execute contract
/// opcodes. This means its storage backend for predicate execution shouldn't provide any
/// functionality.
#[derive(Debug, Default, Clone, Copy)]
pub struct PredicateStorage;

impl<Type: Mappable> StorageInspect<Type> for PredicateStorage {
    type Error = RuntimeError;

    fn get(
        &self,
        _key: &Type::Key,
    ) -> Result<Option<Cow<'_, Type::OwnedValue>>, RuntimeError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }

    fn contains_key(&self, _key: &Type::Key) -> Result<bool, RuntimeError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }
}

impl<Type: Mappable> StorageMutate<Type> for PredicateStorage {
    fn insert(
        &mut self,
        _key: &Type::Key,
        _value: &Type::Value,
    ) -> Result<Option<Type::OwnedValue>, RuntimeError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }

    fn remove(
        &mut self,
        _key: &Type::Key,
    ) -> Result<Option<Type::OwnedValue>, RuntimeError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }
}

impl StorageSize<ContractsRawCode> for PredicateStorage {
    fn size_of_value(&self, _key: &ContractId) -> Result<Option<usize>, RuntimeError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }
}

impl StorageRead<ContractsRawCode> for PredicateStorage {
    fn read(
        &self,
        _key: &<ContractsRawCode as Mappable>::Key,
        _buf: &mut [u8],
    ) -> Result<Option<usize>, Self::Error> {
        Err(RuntimeError::INVALID_PREDICATE)
    }

    fn read_alloc(
        &self,
        _key: &<ContractsRawCode as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(RuntimeError::INVALID_PREDICATE)
    }
}

impl<Key, Type: Mappable> MerkleRootStorage<Key, Type> for PredicateStorage {
    fn root(&self, _parent: &Key) -> Result<MerkleRoot, RuntimeError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }
}

impl ContractsAssetsStorage for PredicateStorage {}

impl InterpreterStorage for PredicateStorage {
    type DataError = RuntimeError;

    fn block_height(&self) -> Result<BlockHeight, RuntimeError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }

    fn timestamp(&self, _height: BlockHeight) -> Result<Word, Self::DataError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }

    fn block_hash(&self, _block_height: BlockHeight) -> Result<Bytes32, RuntimeError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }

    fn coinbase(&self) -> Result<Address, RuntimeError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }

    fn merkle_contract_state_range(
        &self,
        _id: &ContractId,
        _start_key: &Bytes32,
        _range: Word,
    ) -> Result<Vec<Option<Cow<Bytes32>>>, Self::DataError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }

    fn merkle_contract_state_insert_range(
        &mut self,
        _contract: &ContractId,
        _start_key: &Bytes32,
        _values: &[Bytes32],
    ) -> Result<Option<()>, Self::DataError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }

    fn merkle_contract_state_remove_range(
        &mut self,
        _contract: &ContractId,
        _start_key: &Bytes32,
        _range: Word,
    ) -> Result<Option<()>, Self::DataError> {
        Err(RuntimeError::INVALID_PREDICATE)
    }
}

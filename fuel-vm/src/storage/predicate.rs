use alloc::{
    borrow::Cow,
    vec::Vec,
};

use crate::{
    prelude::{
        InterpreterError,
        RuntimeError,
    },
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

/// Storage is unavailable in predicate context.
#[derive(Debug, Clone, Copy)]
pub struct StorageUnavailable;

impl Into<InterpreterError<StorageUnavailable>> for StorageUnavailable {
    fn into(self) -> InterpreterError<StorageUnavailable> {
        let rt: RuntimeError<StorageUnavailable> = self.into();
        rt.into()
    }
}

impl Into<RuntimeError<StorageUnavailable>> for StorageUnavailable {
    fn into(self) -> RuntimeError<StorageUnavailable> {
        RuntimeError::Storage(self)
    }
}

impl<Type: Mappable> StorageInspect<Type> for PredicateStorage {
    type Error = StorageUnavailable;

    fn get(
        &self,
        _key: &Type::Key,
    ) -> Result<Option<Cow<'_, Type::OwnedValue>>, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn contains_key(&self, _key: &Type::Key) -> Result<bool, StorageUnavailable> {
        Err(StorageUnavailable)
    }
}

impl<Type: Mappable> StorageMutate<Type> for PredicateStorage {
    fn insert(
        &mut self,
        _key: &Type::Key,
        _value: &Type::Value,
    ) -> Result<Option<Type::OwnedValue>, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn remove(
        &mut self,
        _key: &Type::Key,
    ) -> Result<Option<Type::OwnedValue>, StorageUnavailable> {
        Err(StorageUnavailable)
    }
}

impl StorageSize<ContractsRawCode> for PredicateStorage {
    fn size_of_value(
        &self,
        _key: &ContractId,
    ) -> Result<Option<usize>, StorageUnavailable> {
        Err(StorageUnavailable)
    }
}

impl StorageRead<ContractsRawCode> for PredicateStorage {
    fn read(
        &self,
        _key: &<ContractsRawCode as Mappable>::Key,
        _buf: &mut [u8],
    ) -> Result<Option<usize>, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn read_alloc(
        &self,
        _key: &<ContractsRawCode as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, StorageUnavailable> {
        Err(StorageUnavailable)
    }
}

impl<Key, Type: Mappable> MerkleRootStorage<Key, Type> for PredicateStorage {
    fn root(&self, _parent: &Key) -> Result<MerkleRoot, StorageUnavailable> {
        Err(StorageUnavailable)
    }
}

impl ContractsAssetsStorage for PredicateStorage {}

impl InterpreterStorage for PredicateStorage {
    type DataError = StorageUnavailable;

    fn block_height(&self) -> Result<BlockHeight, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn timestamp(&self, _height: BlockHeight) -> Result<Word, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn block_hash(
        &self,
        _block_height: BlockHeight,
    ) -> Result<Bytes32, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn coinbase(&self) -> Result<Address, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn merkle_contract_state_range(
        &self,
        _id: &ContractId,
        _start_key: &Bytes32,
        _range: Word,
    ) -> Result<Vec<Option<Cow<Bytes32>>>, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn merkle_contract_state_insert_range(
        &mut self,
        _contract: &ContractId,
        _start_key: &Bytes32,
        _values: &[Bytes32],
    ) -> Result<Option<()>, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn merkle_contract_state_remove_range(
        &mut self,
        _contract: &ContractId,
        _start_key: &Bytes32,
        _range: Word,
    ) -> Result<Option<()>, StorageUnavailable> {
        Err(StorageUnavailable)
    }
}

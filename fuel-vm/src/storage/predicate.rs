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
    StorageInspect,
    StorageMutate,
    StorageRead,
    StorageSize,
    StorageWrite,
};
use fuel_tx::ConsensusParameters;
use fuel_types::{
    BlockHeight,
    Bytes32,
    ContractId,
};

use super::{
    interpreter::ContractsAssetsStorage,
    BlobData,
    ContractsRawCode,
    ContractsState,
    ContractsStateData,
};

/// No-op storage used for predicate operations.
///
/// The storage implementations are expected to provide KV-like operations for contract
/// operations. However, predicates, as defined in the protocol, cannot execute contract
/// opcodes. This means its storage backend for predicate execution shouldn't provide any
/// functionality.
///
/// TODO: blob storage should be implemented
#[derive(Debug, Default, Clone, Copy)]
pub struct PredicateStorage;

/// Storage is unavailable in predicate context.
#[derive(Debug, Clone, Copy)]
pub struct StorageUnavailable;

impl From<StorageUnavailable> for InterpreterError<StorageUnavailable> {
    fn from(val: StorageUnavailable) -> Self {
        let rt: RuntimeError<StorageUnavailable> = val.into();
        rt.into()
    }
}

impl From<StorageUnavailable> for RuntimeError<StorageUnavailable> {
    fn from(val: StorageUnavailable) -> Self {
        RuntimeError::Storage(val)
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
    fn replace(
        &mut self,
        _key: &Type::Key,
        _value: &Type::Value,
    ) -> Result<Option<Type::OwnedValue>, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn take(
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

impl StorageWrite<ContractsRawCode> for PredicateStorage {
    fn write_bytes(
        &mut self,
        _key: &<ContractsRawCode as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<usize, Self::Error> {
        Err(StorageUnavailable)
    }

    fn replace_bytes(
        &mut self,
        _key: &<ContractsRawCode as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<(usize, Option<Vec<u8>>), Self::Error> {
        Err(StorageUnavailable)
    }

    fn take_bytes(
        &mut self,
        _key: &<ContractsRawCode as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(StorageUnavailable)
    }
}

impl StorageSize<ContractsState> for PredicateStorage {
    fn size_of_value(
        &self,
        _key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<usize>, StorageUnavailable> {
        Err(StorageUnavailable)
    }
}

impl StorageRead<ContractsState> for PredicateStorage {
    fn read(
        &self,
        _key: &<ContractsState as Mappable>::Key,
        _buf: &mut [u8],
    ) -> Result<Option<usize>, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn read_alloc(
        &self,
        _key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, StorageUnavailable> {
        Err(StorageUnavailable)
    }
}

impl StorageWrite<ContractsState> for PredicateStorage {
    fn write_bytes(
        &mut self,
        _key: &<ContractsState as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<usize, Self::Error> {
        Err(StorageUnavailable)
    }

    fn replace_bytes(
        &mut self,
        _key: &<ContractsState as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<(usize, Option<Vec<u8>>), Self::Error> {
        Err(StorageUnavailable)
    }

    fn take_bytes(
        &mut self,
        _key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(StorageUnavailable)
    }
}

impl StorageSize<BlobData> for PredicateStorage {
    fn size_of_value(
        &self,
        _key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<usize>, StorageUnavailable> {
        Err(StorageUnavailable)
    }
}

impl StorageRead<BlobData> for PredicateStorage {
    fn read(
        &self,
        _key: &<BlobData as Mappable>::Key,
        _buf: &mut [u8],
    ) -> Result<Option<usize>, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn read_alloc(
        &self,
        _key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, StorageUnavailable> {
        Err(StorageUnavailable)
    }
}

impl StorageWrite<BlobData> for PredicateStorage {
    fn write_bytes(
        &mut self,
        _key: &<BlobData as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<usize, Self::Error> {
        Err(StorageUnavailable)
    }

    fn replace_bytes(
        &mut self,
        _key: &<BlobData as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<(usize, Option<Vec<u8>>), Self::Error> {
        Err(StorageUnavailable)
    }

    fn take_bytes(
        &mut self,
        _key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(StorageUnavailable)
    }
}

impl ContractsAssetsStorage for PredicateStorage {}

impl InterpreterStorage for PredicateStorage {
    type DataError = StorageUnavailable;

    fn block_height(&self) -> Result<BlockHeight, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn consensus_parameters_version(&self) -> Result<u32, Self::DataError> {
        Err(StorageUnavailable)
    }

    fn state_transition_version(&self) -> Result<u32, Self::DataError> {
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

    fn coinbase(&self) -> Result<ContractId, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn set_consensus_parameters(
        &mut self,
        _version: u32,
        _consensus_parameters: &ConsensusParameters,
    ) -> Result<Option<ConsensusParameters>, Self::DataError> {
        Err(StorageUnavailable)
    }

    fn set_state_transition_bytecode(
        &mut self,
        _version: u32,
        _hash: &Bytes32,
    ) -> Result<Option<Bytes32>, Self::DataError> {
        Err(StorageUnavailable)
    }

    fn contract_state_range(
        &self,
        _id: &ContractId,
        _start_key: &Bytes32,
        _range: usize,
    ) -> Result<Vec<Option<Cow<ContractsStateData>>>, StorageUnavailable> {
        Err(StorageUnavailable)
    }

    fn contract_state_insert_range<'a, I>(
        &mut self,
        _: &ContractId,
        _: &Bytes32,
        _: I,
    ) -> Result<usize, Self::DataError>
    where
        I: Iterator<Item = &'a [u8]>,
    {
        Err(StorageUnavailable)
    }

    fn contract_state_remove_range(
        &mut self,
        _contract: &ContractId,
        _start_key: &Bytes32,
        _range: usize,
    ) -> Result<Option<()>, StorageUnavailable> {
        Err(StorageUnavailable)
    }
}

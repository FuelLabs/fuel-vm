use crate::{
    prelude::{
        InterpreterError,
        RuntimeError,
    },
    storage::InterpreterStorage,
};
use alloc::{
    borrow::Cow,
    vec::Vec,
};
use core::fmt::Debug;

use fuel_asm::Word;
use fuel_storage::{
    Mappable,
    StorageAsMut,
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
/// functionality. Unless the storage access is limited to immutable data and read-only.
#[derive(Debug, Default, Clone, Copy)]
pub struct PredicateStorage<D: PredicateBlobStorage> {
    storage: D,
}

impl<D: PredicateBlobStorage> PredicateStorage<D> {
    pub fn new(storage: D) -> Self {
        Self { storage }
    }
}

pub trait PredicateBlobStorage: StorageRead<BlobData> + Clone
where
    Self::Error: Debug,
{
}

#[derive(Debug, Clone, Copy)]
pub enum PredicateStorageError<E> {
    /// Storage operation is unavailable in predicate context.
    UnsupportedStorageOperation,
    /// An storage error occurred
    WrappedError(E),
}

impl<E> From<PredicateStorageError<E>> for InterpreterError<PredicateStorageError<E>> {
    fn from(val: PredicateStorageError<E>) -> Self {
        let rt: RuntimeError<StorageUnavailable> = val.into();
        rt.into()
    }
}

impl<E> From<PredicateStorageError<E>> for RuntimeError<PredicateStorageError<E>> {
    fn from(val: PredicateStorageError<E>) -> Self {
        RuntimeError::Storage(val)
    }
}

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

impl<Type, D> StorageInspect<Type> for PredicateStorage<D>
where
    Type: Mappable,
    D: StorageRead<BlobData>,
{
    type Error = PredicateStorageError<D::Error>;

    fn get(
        &self,
        _key: &Type::Key,
    ) -> Result<Option<Cow<'_, Type::OwnedValue>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn contains_key(&self, _key: &Type::Key) -> Result<bool, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<Type, D> StorageMutate<Type> for PredicateStorage<D>
where
    Type: Mappable,
    D: StorageRead<BlobData>,
{
    fn replace(
        &mut self,
        _key: &Type::Key,
        _value: &Type::Value,
    ) -> Result<Option<Type::OwnedValue>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn take(
        &mut self,
        _key: &Type::Key,
    ) -> Result<Option<Type::OwnedValue>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> StorageSize<ContractsRawCode> for PredicateStorage<D>
where
    D: StorageRead<BlobData>,
{
    fn size_of_value(&self, _key: &ContractId) -> Result<Option<usize>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> StorageRead<ContractsRawCode> for PredicateStorage<D>
where
    D: StorageRead<BlobData>,
{
    fn read(
        &self,
        _key: &<ContractsRawCode as Mappable>::Key,
        _buf: &mut [u8],
    ) -> Result<Option<usize>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn read_alloc(
        &self,
        _key: &<ContractsRawCode as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> StorageWrite<ContractsRawCode> for PredicateStorage<D>
where
    D: StorageRead<BlobData>,
{
    fn write_bytes(
        &mut self,
        _key: &<ContractsRawCode as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<usize, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn replace_bytes(
        &mut self,
        _key: &<ContractsRawCode as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<(usize, Option<Vec<u8>>), Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn take_bytes(
        &mut self,
        _key: &<ContractsRawCode as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> StorageSize<ContractsState> for PredicateStorage<D>
where
    D: StorageRead<BlobData>,
{
    fn size_of_value(
        &self,
        _key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<usize>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> StorageRead<ContractsState> for PredicateStorage<D>
where
    D: StorageRead<BlobData>,
{
    fn read(
        &self,
        _key: &<ContractsState as Mappable>::Key,
        _buf: &mut [u8],
    ) -> Result<Option<usize>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn read_alloc(
        &self,
        _key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> StorageWrite<ContractsState> for PredicateStorage<D>
where
    D: StorageRead<BlobData>,
{
    fn write_bytes(
        &mut self,
        _key: &<ContractsState as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<usize, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn replace_bytes(
        &mut self,
        _key: &<ContractsState as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<(usize, Option<Vec<u8>>), Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn take_bytes(
        &mut self,
        _key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> StorageSize<BlobData> for PredicateStorage<D>
where
    D: StorageRead<BlobData>,
{
    fn size_of_value(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<usize>, Self::Error> {
        StorageSize::<BlobData>::size_of_value(&self.storage, key)
            .map_err(|e| Self::Error::WrappedError(e))
    }
}

impl<D> StorageRead<BlobData> for PredicateStorage<D>
where
    D: StorageRead<BlobData>,
{
    fn read(
        &self,
        key: &<BlobData as Mappable>::Key,
        buf: &mut [u8],
    ) -> Result<Option<usize>, Self::Error> {
        StorageRead::<BlobData>::read(&self.storage, key, buf)
            .map_err(|e| Self::Error::WrappedError(e))
    }

    fn read_alloc(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        StorageRead::<BlobData>::read_alloc(&self.storage, key)
            .map_err(|e| Self::Error::WrappedError(e))
    }
}

impl<D> StorageWrite<BlobData> for PredicateStorage<D>
where
    D: StorageRead<BlobData>,
{
    fn write_bytes(
        &mut self,
        _key: &<BlobData as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<usize, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn replace_bytes(
        &mut self,
        _key: &<BlobData as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<(usize, Option<Vec<u8>>), Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn take_bytes(
        &mut self,
        _key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> ContractsAssetsStorage for PredicateStorage<D> where D: StorageRead<BlobData> {}

impl<D> InterpreterStorage for PredicateStorage<D>
where
    D: StorageRead<BlobData>,
    D::Error: Debug,
{
    type DataError = PredicateStorageError<D::Error>;

    fn block_height(&self) -> Result<BlockHeight, Self::DataError> {
        Err(Self::DataError::UnsupportedStorageOperation)
    }

    fn consensus_parameters_version(&self) -> Result<u32, Self::DataError> {
        Err(Self::DataError::UnsupportedStorageOperation)
    }

    fn state_transition_version(&self) -> Result<u32, Self::DataError> {
        Err(Self::DataError::UnsupportedStorageOperation)
    }

    fn timestamp(&self, _height: BlockHeight) -> Result<Word, Self::DataError> {
        Err(Self::DataError::UnsupportedStorageOperation)
    }

    fn block_hash(&self, _block_height: BlockHeight) -> Result<Bytes32, Self::DataError> {
        Err(Self::DataError::UnsupportedStorageOperation)
    }

    fn coinbase(&self) -> Result<ContractId, Self::DataError> {
        Err(Self::DataError::UnsupportedStorageOperation)
    }

    fn set_consensus_parameters(
        &mut self,
        _version: u32,
        _consensus_parameters: &ConsensusParameters,
    ) -> Result<Option<ConsensusParameters>, Self::DataError> {
        Err(Self::DataError::UnsupportedStorageOperation)
    }

    fn set_state_transition_bytecode(
        &mut self,
        _version: u32,
        _hash: &Bytes32,
    ) -> Result<Option<Bytes32>, Self::DataError> {
        Err(Self::DataError::UnsupportedStorageOperation)
    }

    fn contract_state_range(
        &self,
        _id: &ContractId,
        _start_key: &Bytes32,
        _range: usize,
    ) -> Result<Vec<Option<Cow<ContractsStateData>>>, Self::DataError> {
        Err(Self::DataError::UnsupportedStorageOperation)
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
        Err(Self::DataError::UnsupportedStorageOperation)
    }

    fn contract_state_remove_range(
        &mut self,
        _contract: &ContractId,
        _start_key: &Bytes32,
        _range: usize,
    ) -> Result<Option<()>, Self::DataError> {
        Err(Self::DataError::UnsupportedStorageOperation)
    }
}

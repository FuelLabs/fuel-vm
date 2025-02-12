//! The module contains storage requirements for the predicate execution.

use crate::{
    prelude::{
        InterpreterError,
        RuntimeError,
    },
    storage::InterpreterStorage,
};
use alloc::{
    borrow::Cow,
    string::String,
    vec::Vec,
};
use core::fmt::Debug;

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
    BlobId,
    BlockHeight,
    Bytes32,
    ContractId,
};

use super::{
    interpreter::ContractsAssetsStorage,
    BlobData,
    ContractsAssets,
    ContractsRawCode,
    ContractsState,
    ContractsStateData,
    UploadedBytecodes,
};

/// Create an empty predicate storage.
pub fn empty_predicate_storage() -> PredicateStorage<EmptyStorage> {
    PredicateStorage::new(EmptyStorage)
}

/// No-op storage used for predicate operations.
///
/// The storage implementations are expected to provide KV-like operations for contract
/// operations. However, predicates, as defined in the protocol, cannot execute contract
/// opcodes. This means its storage backend for predicate execution shouldn't provide any
/// functionality. Unless the storage access is limited to immutable data and read-only.
#[derive(Debug, Default)]
pub struct PredicateStorage<D> {
    storage: D,
}

impl<D> PredicateStorage<D> {
    /// instantiate predicate storage with access to Blobs
    pub fn new(storage: D) -> Self {
        Self { storage }
    }
}

/// Errors that happen when using predicate storage
#[derive(Debug, Clone)]
pub enum PredicateStorageError {
    /// Storage operation is unavailable in predicate context.
    UnsupportedStorageOperation,
    /// An storage error occurred
    StorageError(String),
}

impl From<PredicateStorageError> for InterpreterError<PredicateStorageError> {
    fn from(val: PredicateStorageError) -> Self {
        let rt: RuntimeError<PredicateStorageError> = val.into();
        rt.into()
    }
}

impl From<PredicateStorageError> for RuntimeError<PredicateStorageError> {
    fn from(val: PredicateStorageError) -> Self {
        RuntimeError::Storage(val)
    }
}

/// Storage requirements for predicates.
pub trait PredicateStorageRequirements
where
    Self: StorageRead<BlobData>,
{
    /// Converts the storage error to a string.
    fn storage_error_to_string(error: Self::Error) -> String;
}

impl<D> PredicateStorageRequirements for &D
where
    D: PredicateStorageRequirements,
{
    fn storage_error_to_string(error: Self::Error) -> String {
        D::storage_error_to_string(error)
    }
}

/// The type that returns the predicate storage instance.
pub trait PredicateStorageProvider: Sync {
    /// The storage type.
    type Storage: PredicateStorageRequirements + Send + Sync + 'static;

    /// Returns the storage instance.
    fn storage(&self) -> Self::Storage;
}

/// Empty storage.
#[derive(Default, Debug, Clone, Copy)]
pub struct EmptyStorage;

impl StorageInspect<BlobData> for EmptyStorage {
    type Error = PredicateStorageError;

    fn get(
        &self,
        _: &BlobId,
    ) -> Result<Option<Cow<<BlobData as Mappable>::OwnedValue>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn contains_key(&self, _: &BlobId) -> Result<bool, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl StorageSize<BlobData> for EmptyStorage {
    fn size_of_value(&self, _: &BlobId) -> Result<Option<usize>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl StorageRead<BlobData> for EmptyStorage {
    fn read(&self, _: &BlobId, _: usize, _: &mut [u8]) -> Result<bool, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn read_alloc(&self, _: &BlobId) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl PredicateStorageRequirements for EmptyStorage {
    fn storage_error_to_string(error: Self::Error) -> String {
        alloc::format!("{:?}", error)
    }
}

impl PredicateStorageProvider for EmptyStorage {
    type Storage = Self;

    fn storage(&self) -> Self::Storage {
        *self
    }
}

trait NoStorage {}

impl NoStorage for ContractsState {}
impl NoStorage for ContractsRawCode {}
impl NoStorage for ContractsAssets {}
impl NoStorage for UploadedBytecodes {}

impl<Type, D> StorageInspect<Type> for PredicateStorage<D>
where
    Type: Mappable + NoStorage,
{
    type Error = PredicateStorageError;

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

impl<D> StorageInspect<BlobData> for PredicateStorage<D>
where
    D: PredicateStorageRequirements,
{
    type Error = PredicateStorageError;

    fn get(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Cow<'_, <BlobData as Mappable>::OwnedValue>>, Self::Error> {
        <D as StorageInspect<BlobData>>::get(&self.storage, key)
            .map_err(|e| Self::Error::StorageError(D::storage_error_to_string(e)))
    }

    fn contains_key(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<bool, Self::Error> {
        <D as StorageInspect<BlobData>>::contains_key(&self.storage, key)
            .map_err(|e| Self::Error::StorageError(D::storage_error_to_string(e)))
    }
}

impl<Type, D> StorageMutate<Type> for PredicateStorage<D>
where
    Type: Mappable,
    Self: StorageInspect<Type, Error = PredicateStorageError>,
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

impl<D> StorageSize<ContractsRawCode> for PredicateStorage<D> {
    fn size_of_value(&self, _key: &ContractId) -> Result<Option<usize>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> StorageRead<ContractsRawCode> for PredicateStorage<D> {
    fn read(
        &self,
        _key: &<ContractsRawCode as Mappable>::Key,
        _offset: usize,
        _buf: &mut [u8],
    ) -> Result<bool, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn read_alloc(
        &self,
        _key: &<ContractsRawCode as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> StorageWrite<ContractsRawCode> for PredicateStorage<D> {
    fn write_bytes(
        &mut self,
        _key: &<ContractsRawCode as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<(), Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn replace_bytes(
        &mut self,
        _key: &<ContractsRawCode as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn take_bytes(
        &mut self,
        _key: &<ContractsRawCode as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> StorageSize<ContractsState> for PredicateStorage<D> {
    fn size_of_value(
        &self,
        _key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<usize>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> StorageRead<ContractsState> for PredicateStorage<D> {
    fn read(
        &self,
        _key: &<ContractsState as Mappable>::Key,
        _offset: usize,
        _buf: &mut [u8],
    ) -> Result<bool, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn read_alloc(
        &self,
        _key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> StorageWrite<ContractsState> for PredicateStorage<D> {
    fn write_bytes(
        &mut self,
        _key: &<ContractsState as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<(), Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn replace_bytes(
        &mut self,
        _key: &<ContractsState as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<Option<Vec<u8>>, Self::Error> {
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
    D: PredicateStorageRequirements,
{
    fn size_of_value(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<usize>, Self::Error> {
        StorageSize::<BlobData>::size_of_value(&self.storage, key)
            .map_err(|e| Self::Error::StorageError(D::storage_error_to_string(e)))
    }
}

impl<D> StorageRead<BlobData> for PredicateStorage<D>
where
    D: PredicateStorageRequirements,
{
    fn read(
        &self,
        key: &<BlobData as Mappable>::Key,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<bool, Self::Error> {
        StorageRead::<BlobData>::read(&self.storage, key, offset, buf)
            .map_err(|e| Self::Error::StorageError(D::storage_error_to_string(e)))
    }

    fn read_alloc(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        StorageRead::<BlobData>::read_alloc(&self.storage, key)
            .map_err(|e| Self::Error::StorageError(D::storage_error_to_string(e)))
    }
}

impl<D> StorageWrite<BlobData> for PredicateStorage<D>
where
    D: PredicateStorageRequirements,
{
    fn write_bytes(
        &mut self,
        _key: &<BlobData as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<(), Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn replace_bytes(
        &mut self,
        _key: &<BlobData as Mappable>::Key,
        _buf: &[u8],
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }

    fn take_bytes(
        &mut self,
        _key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Err(Self::Error::UnsupportedStorageOperation)
    }
}

impl<D> ContractsAssetsStorage for PredicateStorage<D> {}

impl<D> InterpreterStorage for PredicateStorage<D>
where
    D: PredicateStorageRequirements,
{
    type DataError = PredicateStorageError;

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

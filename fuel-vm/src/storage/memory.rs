#![allow(clippy::cast_possible_truncation)]

use crate::{
    error::{
        InterpreterError,
        RuntimeError,
    },
    storage::{
        ContractsAssetKey,
        ContractsAssets,
        ContractsRawCode,
        ContractsState,
        ContractsStateData,
        ContractsStateKey,
        InterpreterStorage,
        UploadedBytecode,
        UploadedBytecodes,
        interpreter::ContractsAssetsStorage,
    },
};

use fuel_crypto::Hasher;
use fuel_storage::{
    Mappable,
    StorageAsMut,
    StorageAsRef,
    StorageInspect,
    StorageMutate,
    StorageRead,
    StorageSize,
    StorageWrite,
};
use fuel_tx::{
    ConsensusParameters,
    Contract,
};
use fuel_types::{
    BlobId,
    BlockHeight,
    Bytes32,
    ContractId,
    Word,
};
use tai64::Tai64;

use super::{
    BlobBytes,
    BlobData,
};

use crate::storage::predicate::PredicateStorageRequirements;
use alloc::{
    borrow::Cow,
    collections::BTreeMap,
    vec::Vec,
};

/// Errors arising from accessing the memory storage.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum MemoryStorageError {
    /// The offset specified for the serialized value exceeds its length
    #[display(fmt = "Offset {_0} is greater than the length of the value {_1}")]
    OffsetOutOfBounds(usize, usize),
}

impl From<MemoryStorageError> for RuntimeError<MemoryStorageError> {
    fn from(e: MemoryStorageError) -> Self {
        RuntimeError::Storage(e)
    }
}

impl From<MemoryStorageError> for InterpreterError<MemoryStorageError> {
    fn from(e: MemoryStorageError) -> Self {
        InterpreterError::Storage(e)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct MemoryStorageInner {
    contracts: BTreeMap<ContractId, Contract>,
    balances: BTreeMap<ContractsAssetKey, Word>,
    contract_state: BTreeMap<ContractsStateKey, ContractsStateData>,
    blobs: BTreeMap<BlobId, BlobBytes>,
    /// Mapping from consensus parameters version to consensus parameters.
    consensus_parameters_versions: BTreeMap<u32, ConsensusParameters>,
    /// Mapping from state transition bytecode root to bytecode.
    state_transition_bytecodes: BTreeMap<Bytes32, UploadedBytecode>,
    /// Mapping from state transition bytecode version to hash.
    state_transition_bytecodes_versions: BTreeMap<u32, Bytes32>,
}

#[derive(Debug, Clone)]
/// In-memory storage implementation for the interpreter.
///
/// It tracks 3 states:
///
/// - memory: the transactions will be applied to this state.
/// - transacted: will receive the committed `memory` state.
/// - persisted: will receive the persisted `transacted` state.
pub struct MemoryStorage {
    block_height: BlockHeight,
    coinbase: ContractId,
    consensus_parameters_version: u32,
    state_transition_version: u32,
    memory: MemoryStorageInner,
    transacted: MemoryStorageInner,
    persisted: MemoryStorageInner,
}

impl MemoryStorage {
    /// Create a new memory storage.
    pub fn new(block_height: BlockHeight, coinbase: ContractId) -> Self {
        Self::new_with_versions(block_height, coinbase, 0, 0)
    }

    /// Create a new memory storage with versions.
    pub fn new_with_versions(
        block_height: BlockHeight,
        coinbase: ContractId,
        consensus_parameters_version: u32,
        state_transition_version: u32,
    ) -> Self {
        Self {
            block_height,
            coinbase,
            consensus_parameters_version,
            state_transition_version,
            memory: Default::default(),
            transacted: Default::default(),
            persisted: Default::default(),
        }
    }

    /// Iterate over all contract state in storage
    pub fn all_contract_state(
        &self,
    ) -> impl Iterator<Item = (&ContractsStateKey, &ContractsStateData)> {
        self.memory.contract_state.iter()
    }

    /// Fetch a mapping from the contract state.
    pub fn contract_state(
        &self,
        contract: &ContractId,
        key: &Bytes32,
    ) -> Cow<'_, ContractsStateData> {
        self.storage::<ContractsState>()
            .get(&(contract, key).into())
            .expect("Infallible")
            .unwrap_or(Cow::Owned(ContractsStateData::default()))
    }

    /// Set the transacted state to the memory state.
    pub fn commit(&mut self) {
        self.transacted = self.memory.clone();
    }

    /// Revert the memory state to the transacted state.
    pub fn revert(&mut self) {
        self.memory = self.transacted.clone();
    }

    /// Revert the memory and transacted changes to the persisted state.
    pub fn rollback(&mut self) {
        self.memory = self.persisted.clone();
        self.transacted = self.persisted.clone();
    }

    /// Persist the changes from transacted to memory+persisted state.
    pub fn persist(&mut self) {
        self.memory = self.transacted.clone();
        self.persisted = self.transacted.clone();
    }

    #[cfg(feature = "test-helpers")]
    /// Set the block height of the chain
    pub fn set_block_height(&mut self, block_height: BlockHeight) {
        self.block_height = block_height;
    }

    #[cfg(feature = "test-helpers")]
    /// Set the consensus parameters version
    pub fn set_consensus_parameters_version(
        &mut self,
        consensus_parameters_version: u32,
    ) {
        self.consensus_parameters_version = consensus_parameters_version;
    }

    #[cfg(feature = "test-helpers")]
    /// Set the state transition version
    pub fn set_state_transition_version(&mut self, state_transition_version: u32) {
        self.state_transition_version = state_transition_version;
    }

    #[cfg(feature = "test-helpers")]
    /// Returns mutable reference to the consensus parameters versions table.
    pub fn consensus_parameters_versions_mut(
        &mut self,
    ) -> &mut BTreeMap<u32, ConsensusParameters> {
        &mut self.memory.consensus_parameters_versions
    }

    #[cfg(feature = "test-helpers")]
    /// Returns mutable reference to the state transition bytecodes table.
    pub fn state_transition_bytecodes_mut(
        &mut self,
    ) -> &mut BTreeMap<Bytes32, UploadedBytecode> {
        &mut self.memory.state_transition_bytecodes
    }

    #[cfg(feature = "test-helpers")]
    /// Returns mutable reference to the state transition bytecodes versions table.
    pub fn state_transition_bytecodes_versions_mut(
        &mut self,
    ) -> &mut BTreeMap<u32, Bytes32> {
        &mut self.memory.state_transition_bytecodes_versions
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        let block_height = 1.into();
        let coinbase = ContractId::from(*Hasher::hash(b"coinbase"));

        Self::new(block_height, coinbase)
    }
}

impl StorageInspect<ContractsRawCode> for MemoryStorage {
    type Error = MemoryStorageError;

    fn get(&self, key: &ContractId) -> Result<Option<Cow<'_, Contract>>, Self::Error> {
        Ok(self.memory.contracts.get(key).map(Cow::Borrowed))
    }

    fn contains_key(&self, key: &ContractId) -> Result<bool, Self::Error> {
        Ok(self.memory.contracts.contains_key(key))
    }
}

impl StorageMutate<ContractsRawCode> for MemoryStorage {
    fn replace(
        &mut self,
        key: &ContractId,
        value: &[u8],
    ) -> Result<Option<Contract>, Self::Error> {
        Ok(self.memory.contracts.insert(*key, value.to_vec().into()))
    }

    fn take(&mut self, key: &ContractId) -> Result<Option<Contract>, Self::Error> {
        Ok(self.memory.contracts.remove(key))
    }
}

impl StorageWrite<ContractsRawCode> for MemoryStorage {
    fn write_bytes(&mut self, key: &ContractId, buf: &[u8]) -> Result<(), Self::Error> {
        self.memory
            .contracts
            .insert(*key, Contract::from(buf.to_vec()));
        Ok(())
    }

    fn replace_bytes(
        &mut self,
        key: &ContractId,
        buf: &[u8],
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self
            .memory
            .contracts
            .insert(*key, Contract::from(buf.to_vec()))
            .map(Into::into))
    }

    fn take_bytes(&mut self, key: &ContractId) -> Result<Option<Vec<u8>>, Self::Error> {
        let prev = self.memory.contracts.remove(key).map(Into::into);
        Ok(prev)
    }
}

impl StorageSize<ContractsRawCode> for MemoryStorage {
    fn size_of_value(&self, key: &ContractId) -> Result<Option<usize>, Self::Error> {
        Ok(self.memory.contracts.get(key).map(|c| c.as_ref().len()))
    }
}

impl StorageRead<ContractsRawCode> for MemoryStorage {
    fn read(
        &self,
        key: &ContractId,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<bool, Self::Error> {
        if let Some(c) = self.memory.contracts.get(key) {
            let contract_len = c.as_ref().len();
            let start = offset;
            let end = offset.saturating_add(buf.len());
            // We need to handle the case where the offset is greater than the length
            // of the contract In this case we follow the same
            // approach as `copy_from_slice_zero_fill`
            if end > contract_len {
                return Err(MemoryStorageError::OffsetOutOfBounds(end, contract_len));
            }

            let starting_from_offset = &c.as_ref()[start..end];
            buf[..].copy_from_slice(starting_from_offset);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn read_alloc(&self, key: &ContractId) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.memory.contracts.get(key).map(|c| c.as_ref().to_vec()))
    }
}

impl StorageInspect<UploadedBytecodes> for MemoryStorage {
    type Error = MemoryStorageError;

    fn get(
        &self,
        key: &<UploadedBytecodes as Mappable>::Key,
    ) -> Result<Option<Cow<'_, UploadedBytecode>>, Self::Error> {
        Ok(self
            .memory
            .state_transition_bytecodes
            .get(key)
            .map(Cow::Borrowed))
    }

    fn contains_key(
        &self,
        key: &<UploadedBytecodes as Mappable>::Key,
    ) -> Result<bool, Self::Error> {
        Ok(self.memory.state_transition_bytecodes.contains_key(key))
    }
}

impl StorageMutate<UploadedBytecodes> for MemoryStorage {
    fn replace(
        &mut self,
        key: &<UploadedBytecodes as Mappable>::Key,
        value: &<UploadedBytecodes as Mappable>::Value,
    ) -> Result<Option<UploadedBytecode>, Self::Error> {
        Ok(self
            .memory
            .state_transition_bytecodes
            .insert(*key, value.clone()))
    }

    fn take(
        &mut self,
        key: &<UploadedBytecodes as Mappable>::Key,
    ) -> Result<Option<UploadedBytecode>, Self::Error> {
        Ok(self.memory.state_transition_bytecodes.remove(key))
    }
}

impl StorageInspect<ContractsAssets> for MemoryStorage {
    type Error = MemoryStorageError;

    fn get(
        &self,
        key: &<ContractsAssets as Mappable>::Key,
    ) -> Result<Option<Cow<'_, Word>>, Self::Error> {
        Ok(self.memory.balances.get(key).map(Cow::Borrowed))
    }

    fn contains_key(
        &self,
        key: &<ContractsAssets as Mappable>::Key,
    ) -> Result<bool, Self::Error> {
        Ok(self.memory.balances.contains_key(key))
    }
}

impl StorageMutate<ContractsAssets> for MemoryStorage {
    fn replace(
        &mut self,
        key: &<ContractsAssets as Mappable>::Key,
        value: &Word,
    ) -> Result<Option<Word>, Self::Error> {
        Ok(self.memory.balances.insert(*key, *value))
    }

    fn take(
        &mut self,
        key: &<ContractsAssets as Mappable>::Key,
    ) -> Result<Option<Word>, Self::Error> {
        Ok(self.memory.balances.remove(key))
    }
}

impl StorageInspect<ContractsState> for MemoryStorage {
    type Error = MemoryStorageError;

    fn get(
        &self,
        key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<Cow<'_, <ContractsState as Mappable>::OwnedValue>>, Self::Error>
    {
        Ok(self.memory.contract_state.get(key).map(Cow::Borrowed))
    }

    fn contains_key(
        &self,
        key: &<ContractsState as Mappable>::Key,
    ) -> Result<bool, Self::Error> {
        Ok(self.memory.contract_state.contains_key(key))
    }
}

impl StorageMutate<ContractsState> for MemoryStorage {
    fn replace(
        &mut self,
        key: &<ContractsState as Mappable>::Key,
        value: &<ContractsState as Mappable>::Value,
    ) -> Result<Option<<ContractsState as Mappable>::OwnedValue>, Self::Error> {
        Ok(self
            .memory
            .contract_state
            .insert(*key, value.to_vec().into()))
    }

    fn take(
        &mut self,
        key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<ContractsStateData>, Self::Error> {
        Ok(self.memory.contract_state.remove(key))
    }
}

impl StorageWrite<ContractsState> for MemoryStorage {
    fn write_bytes(
        &mut self,
        key: &<ContractsState as Mappable>::Key,
        buf: &[u8],
    ) -> Result<(), Self::Error> {
        self.memory
            .contract_state
            .insert(*key, ContractsStateData::from(buf.to_vec()));
        Ok(())
    }

    fn replace_bytes(
        &mut self,
        key: &<ContractsState as Mappable>::Key,
        buf: &[u8],
    ) -> Result<Option<Vec<u8>>, Self::Error>
    where
        Self: StorageSize<ContractsState>,
    {
        Ok(self
            .memory
            .contract_state
            .insert(*key, ContractsStateData::from(buf.to_vec()))
            .map(Into::into))
    }

    fn take_bytes(
        &mut self,
        key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        let prev = self.memory.contract_state.remove(key).map(Into::into);
        Ok(prev)
    }
}

impl StorageSize<ContractsState> for MemoryStorage {
    fn size_of_value(
        &self,
        key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<usize>, Self::Error> {
        Ok(self
            .memory
            .contract_state
            .get(key)
            .map(|c| c.as_ref().len()))
    }
}

impl StorageRead<ContractsState> for MemoryStorage {
    fn read(
        &self,
        key: &<ContractsState as Mappable>::Key,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<bool, Self::Error> {
        if let Some(data) = self.memory.contract_state.get(key) {
            let contract_state_len = data.as_ref().len();
            // We need to handle the case where the offset is greater than the length
            // of the serialized ContractState. In this case we follow
            // the same approach as `copy_from_slice_zero_fill` and
            // fill the input buffer with zeros.
            if offset > contract_state_len {
                return Err(MemoryStorageError::OffsetOutOfBounds(
                    offset,
                    contract_state_len,
                ));
            }
            let starting_from_offset = &data.as_ref()[offset..];
            let len = buf.len().min(starting_from_offset.len());
            buf[..len].copy_from_slice(&starting_from_offset[..len]);
            buf[len..].fill(0);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn read_alloc(
        &self,
        key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self
            .memory
            .contract_state
            .get(key)
            .map(|c| c.as_ref().to_vec()))
    }
}

impl StorageSize<BlobData> for MemoryStorage {
    fn size_of_value(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<usize>, Self::Error> {
        Ok(self.memory.blobs.get(key).map(|c| c.as_ref().len()))
    }
}

impl StorageRead<BlobData> for MemoryStorage {
    fn read(
        &self,
        key: &<BlobData as Mappable>::Key,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<bool, Self::Error> {
        if let Some(data) = self.memory.blobs.get(key) {
            let blob_len = data.as_ref().len();
            let start = offset;
            let end = offset.saturating_add(buf.len());
            // We need to handle the case where the offset is greater than the length
            // of the serialized ContractState. In this case we follow
            // the same approach as `copy_from_slice_zero_fill` and
            // fill the input buffer with zeros.
            if end > blob_len {
                return Err(MemoryStorageError::OffsetOutOfBounds(offset, blob_len));
            }

            let starting_from_offset = &data.as_ref()[start..end];
            buf[..].copy_from_slice(starting_from_offset);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn read_alloc(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.memory.blobs.get(key).map(|c| c.as_ref().to_vec()))
    }
}

impl StorageInspect<BlobData> for MemoryStorage {
    type Error = MemoryStorageError;

    fn get(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Cow<'_, <BlobData as Mappable>::OwnedValue>>, Self::Error> {
        Ok(self.memory.blobs.get(key).map(Cow::Borrowed))
    }

    fn contains_key(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<bool, Self::Error> {
        Ok(self.memory.blobs.contains_key(key))
    }
}

impl StorageMutate<BlobData> for MemoryStorage {
    fn replace(
        &mut self,
        key: &<BlobData as Mappable>::Key,
        value: &<BlobData as Mappable>::Value,
    ) -> Result<Option<<BlobData as Mappable>::OwnedValue>, Self::Error> {
        Ok(self.memory.blobs.insert(*key, value.to_vec().into()))
    }

    fn take(
        &mut self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<BlobBytes>, Self::Error> {
        Ok(self.memory.blobs.remove(key))
    }
}

impl StorageWrite<BlobData> for MemoryStorage {
    fn write_bytes(
        &mut self,
        key: &<BlobData as Mappable>::Key,
        buf: &[u8],
    ) -> Result<(), Self::Error> {
        self.memory
            .blobs
            .insert(*key, BlobBytes::from(buf.to_vec()));
        Ok(())
    }

    fn replace_bytes(
        &mut self,
        key: &<BlobData as Mappable>::Key,
        buf: &[u8],
    ) -> Result<Option<Vec<u8>>, Self::Error>
    where
        Self: StorageSize<BlobData>,
    {
        let prev = self
            .memory
            .blobs
            .insert(*key, BlobBytes::from(buf.to_vec()))
            .map(Into::into);
        Ok(prev)
    }

    fn take_bytes(
        &mut self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        let prev = self.memory.blobs.remove(key).map(Into::into);
        Ok(prev)
    }
}

use anyhow::anyhow;

/// The trait around the `U256` type allows increasing the key by one.
pub trait IncreaseStorageKey {
    /// Increases the key by one.
    ///
    /// Returns a `Result::Err` in the case of overflow.
    fn increase(&mut self) -> anyhow::Result<()>;
}

use primitive_types::U256;

impl IncreaseStorageKey for U256 {
    fn increase(&mut self) -> anyhow::Result<()> {
        *self = self
            .checked_add(1.into())
            .ok_or_else(|| anyhow!("range op exceeded available keyspace"))?;
        Ok(())
    }
}

impl ContractsAssetsStorage for MemoryStorage {}

impl InterpreterStorage for MemoryStorage {
    type DataError = MemoryStorageError;

    fn block_height(&self) -> Result<BlockHeight, Self::DataError> {
        Ok(self.block_height)
    }

    fn consensus_parameters_version(&self) -> Result<u32, Self::DataError> {
        Ok(self.consensus_parameters_version)
    }

    fn state_transition_version(&self) -> Result<u32, Self::DataError> {
        Ok(self.state_transition_version)
    }

    #[allow(clippy::arithmetic_side_effects)] // Safety: not enough bits to overflow
    fn timestamp(&self, height: BlockHeight) -> Result<Word, Self::DataError> {
        const GENESIS: Tai64 = Tai64::UNIX_EPOCH;
        const INTERVAL: Word = 10;

        Ok((GENESIS + (*height as Word * INTERVAL)).0)
    }

    fn block_hash(&self, block_height: BlockHeight) -> Result<Bytes32, Self::DataError> {
        Ok(Hasher::hash(block_height.to_be_bytes()))
    }

    fn coinbase(&self) -> Result<ContractId, Self::DataError> {
        Ok(self.coinbase)
    }

    fn set_consensus_parameters(
        &mut self,
        version: u32,
        consensus_parameters: &ConsensusParameters,
    ) -> Result<Option<ConsensusParameters>, Self::DataError> {
        Ok(self
            .memory
            .consensus_parameters_versions
            .insert(version, consensus_parameters.clone()))
    }

    fn set_state_transition_bytecode(
        &mut self,
        version: u32,
        bytecode: &Bytes32,
    ) -> Result<Option<Bytes32>, Self::DataError> {
        Ok(self
            .memory
            .state_transition_bytecodes_versions
            .insert(version, *bytecode))
    }

    fn contract_state_range(
        &self,
        id: &ContractId,
        start_key: &Bytes32,
        range: usize,
    ) -> Result<Vec<Option<Cow<'_, ContractsStateData>>>, Self::DataError> {
        let mut key = primitive_types::U256::from_big_endian(start_key.as_ref());
        let mut state_key = Bytes32::zeroed();

        let mut results = Vec::new();
        for i in 0..range {
            if i != 0 {
                key.increase().unwrap();
            }
            key.to_big_endian(state_key.as_mut());
            let multikey = ContractsStateKey::new(id, &state_key);
            results.push(self.storage::<ContractsState>().get(&multikey)?);
        }
        Ok(results)
    }

    fn contract_state_insert_range<'a, I>(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        values: I,
    ) -> Result<usize, Self::DataError>
    where
        I: Iterator<Item = &'a [u8]>,
    {
        let values: Vec<_> = values.collect();
        let mut current_key = U256::from_big_endian(start_key.as_ref());

        // verify key is in range
        current_key
            .checked_add(U256::from(values.len().saturating_sub(1)))
            .ok_or_else(|| anyhow!("range op exceeded available keyspace"))
            .unwrap();

        let mut key_bytes = Bytes32::zeroed();
        let mut found_unset = 0u32;
        for (idx, value) in values.iter().enumerate() {
            if idx != 0 {
                current_key.increase().unwrap();
            }
            current_key.to_big_endian(key_bytes.as_mut());

            let option = self
                .storage_as_mut::<ContractsState>()
                .replace(&(contract, &key_bytes).into(), value)?;

            if option.is_none() {
                found_unset = found_unset
                    .checked_add(1)
                    .expect("We've checked it above via `values.len()`");
            }
        }

        Ok(found_unset as usize)
    }

    fn contract_state_remove_range(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        range: usize,
    ) -> Result<Option<()>, Self::DataError> {
        let mut found_unset = false;

        let mut current_key = U256::from_big_endian(start_key.as_ref());

        let mut key_bytes = Bytes32::zeroed();
        for i in 0..range {
            if i != 0 {
                current_key.increase().unwrap();
            }
            current_key.to_big_endian(key_bytes.as_mut());

            let option = self
                .storage_as_mut::<ContractsState>()
                .take(&(contract, &key_bytes).into())?;

            found_unset |= option.is_none();
        }

        if found_unset { Ok(None) } else { Ok(Some(())) }
    }
}

impl PredicateStorageRequirements for MemoryStorage {
    fn storage_error_to_string(error: Self::Error) -> alloc::string::String {
        alloc::format!("{:?}", error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use test_case::test_case;

    const fn key(k: u8) -> [u8; 32] {
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, k,
        ]
    }

    #[test_case(&[&[0u8; 32]], &[0u8; 32], 1 => vec![Some(Default::default())])]
    #[test_case(&[&[0u8; 32]], &[0u8; 32], 0 => Vec::<Option<ContractsStateData>>::with_capacity(0))]
    #[test_case(&[], &[0u8; 32], 1 => vec![None])]
    #[test_case(&[], &[1u8; 32], 1 => vec![None])]
    #[test_case(&[&[0u8; 32]], &key(1), 2 => vec![None, None])]
    #[test_case(&[&key(1), &key(3)], &[0u8; 32], 4 => vec![None, Some(Default::default()), None, Some(Default::default())])]
    #[test_case(&[&[0u8; 32], &key(1)], &[0u8; 32], 1 => vec![Some(Default::default())])]
    fn test_contract_state_range(
        store: &[&[u8; 32]],
        start: &[u8; 32],
        range: usize,
    ) -> Vec<Option<ContractsStateData>> {
        let mut mem = MemoryStorage::default();
        for k in store {
            mem.memory.contract_state.insert(
                (&ContractId::default(), &(**k).into()).into(),
                Default::default(),
            );
        }
        mem.contract_state_range(&ContractId::default(), &(*start).into(), range)
            .unwrap()
            .into_iter()
            .map(|v| v.map(|v| v.into_owned()))
            .collect()
    }

    #[test_case(0, 32 => Ok(true))]
    #[test_case(4, 32 => Ok(true))]
    #[test_case(8, 32 => Ok(true))]
    #[test_case(0, 28 => Ok(true))]
    #[test_case(4, 28 => Ok(true))]
    #[test_case(8, 28 => Ok(true))]
    #[test_case(28, 0 => Ok(true))]
    #[test_case(28, 4 => Ok(true))]
    #[test_case(28, 8 => Ok(true))]
    #[test_case(32, 0 => Ok(true))]
    #[test_case(32, 4 => Ok(true))]
    #[test_case(32, 8 => Ok(true))]
    #[test_case(33, 0 => Err(MemoryStorageError::OffsetOutOfBounds(33,32)))]
    #[test_case(33, 4 => Err(MemoryStorageError::OffsetOutOfBounds(33,32)))]
    #[test_case(33, 8 => Err(MemoryStorageError::OffsetOutOfBounds(33,32)))]
    fn test_contract_read(
        offset: usize,
        load_buf_size: usize,
    ) -> Result<bool, MemoryStorageError> {
        // Given
        let raw_contract = [1u8; 32];
        let mut mem = MemoryStorage::default();
        mem.memory
            .contracts
            .insert(ContractId::default(), raw_contract.as_ref().to_vec().into());
        let buf_size = raw_contract.len().saturating_sub(offset).min(load_buf_size);
        let mut buf = vec![0u8; buf_size];

        // When
        let r = StorageRead::<ContractsRawCode>::read(
            &mem,
            &ContractId::default(),
            offset,
            &mut buf,
        );

        // Then

        if r.is_ok() {
            assert!(buf[0..buf_size].iter().all(|&v| v == 1));
            assert!(buf[buf_size..].iter().all(|&v| v == 0));
        }

        r
    }
}

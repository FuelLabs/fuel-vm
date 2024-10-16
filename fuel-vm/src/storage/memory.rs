#![allow(clippy::cast_possible_truncation)]

use crate::storage::{
    ContractsAssetKey,
    ContractsAssets,
    ContractsRawCode,
    ContractsState,
    ContractsStateData,
    ContractsStateKey,
    InterpreterStorage,
    UploadedBytecode,
    UploadedBytecodes,
};

use fuel_crypto::Hasher;
use fuel_storage::{
    Mappable,
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
    interpreter::ContractsAssetsStorage,
    BlobBytes,
    BlobData,
};

use crate::storage::predicate::PredicateStorageRequirements;
use alloc::{
    borrow::Cow,
    collections::BTreeMap,
    vec::Vec,
};
use core::convert::Infallible;

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
    type Error = Infallible;

    fn get(&self, key: &ContractId) -> Result<Option<Cow<'_, Contract>>, Infallible> {
        Ok(self.memory.contracts.get(key).map(Cow::Borrowed))
    }

    fn contains_key(&self, key: &ContractId) -> Result<bool, Infallible> {
        Ok(self.memory.contracts.contains_key(key))
    }
}

impl StorageMutate<ContractsRawCode> for MemoryStorage {
    fn replace(
        &mut self,
        key: &ContractId,
        value: &[u8],
    ) -> Result<Option<Contract>, Infallible> {
        Ok(self.memory.contracts.insert(*key, value.into()))
    }

    fn take(&mut self, key: &ContractId) -> Result<Option<Contract>, Infallible> {
        Ok(self.memory.contracts.remove(key))
    }

    fn replace_forced(
        &mut self,
        _key: &<ContractsRawCode as Mappable>::Key,
        _value: &<ContractsRawCode as Mappable>::Value,
    ) -> Result<Option<<ContractsRawCode as Mappable>::OwnedValue>, Self::Error> {
        unimplemented!();
    }
}

impl StorageWrite<ContractsRawCode> for MemoryStorage {
    fn write_bytes(&mut self, key: &ContractId, buf: &[u8]) -> Result<usize, Infallible> {
        let size = buf.len();
        self.memory.contracts.insert(*key, Contract::from(buf));
        Ok(size)
    }

    fn replace_bytes(
        &mut self,
        key: &ContractId,
        buf: &[u8],
    ) -> Result<(usize, Option<Vec<u8>>), Self::Error> {
        let size = buf.len();
        let prev = self
            .memory
            .contracts
            .insert(*key, Contract::from(buf))
            .map(Into::into);
        Ok((size, prev))
    }

    fn take_bytes(&mut self, key: &ContractId) -> Result<Option<Vec<u8>>, Self::Error> {
        let prev = self.memory.contracts.remove(key).map(Into::into);
        Ok(prev)
    }
}

impl StorageSize<ContractsRawCode> for MemoryStorage {
    fn size_of_value(&self, key: &ContractId) -> Result<Option<usize>, Infallible> {
        Ok(self.memory.contracts.get(key).map(|c| c.as_ref().len()))
    }
}

impl StorageRead<ContractsRawCode> for MemoryStorage {
    fn read(
        &self,
        key: &ContractId,
        buf: &mut [u8],
    ) -> Result<Option<usize>, Self::Error> {
        Ok(self.memory.contracts.get(key).map(|c| {
            let len = buf.len().min(c.as_ref().len());
            buf.copy_from_slice(&c.as_ref()[..len]);
            len
        }))
    }

    fn read_alloc(&self, key: &ContractId) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.memory.contracts.get(key).map(|c| c.as_ref().to_vec()))
    }
}

impl StorageInspect<UploadedBytecodes> for MemoryStorage {
    type Error = Infallible;

    fn get(
        &self,
        key: &<UploadedBytecodes as Mappable>::Key,
    ) -> Result<Option<Cow<'_, UploadedBytecode>>, Infallible> {
        Ok(self
            .memory
            .state_transition_bytecodes
            .get(key)
            .map(Cow::Borrowed))
    }

    fn contains_key(
        &self,
        key: &<UploadedBytecodes as Mappable>::Key,
    ) -> Result<bool, Infallible> {
        Ok(self.memory.state_transition_bytecodes.contains_key(key))
    }
}

impl StorageMutate<UploadedBytecodes> for MemoryStorage {
    fn replace(
        &mut self,
        key: &<UploadedBytecodes as Mappable>::Key,
        value: &<UploadedBytecodes as Mappable>::Value,
    ) -> Result<Option<UploadedBytecode>, Infallible> {
        Ok(self
            .memory
            .state_transition_bytecodes
            .insert(*key, value.clone()))
    }

    fn take(
        &mut self,
        key: &<UploadedBytecodes as Mappable>::Key,
    ) -> Result<Option<UploadedBytecode>, Infallible> {
        Ok(self.memory.state_transition_bytecodes.remove(key))
    }

    fn replace_forced(
        &mut self,
        _key: &<UploadedBytecodes as Mappable>::Key,
        _value: &<UploadedBytecodes as Mappable>::Value,
    ) -> Result<Option<<UploadedBytecodes as Mappable>::OwnedValue>, Self::Error> {
        unimplemented!()
    }
}

impl StorageInspect<ContractsAssets> for MemoryStorage {
    type Error = Infallible;

    fn get(
        &self,
        key: &<ContractsAssets as Mappable>::Key,
    ) -> Result<Option<Cow<'_, Word>>, Infallible> {
        Ok(self.memory.balances.get(key).map(Cow::Borrowed))
    }

    fn contains_key(
        &self,
        key: &<ContractsAssets as Mappable>::Key,
    ) -> Result<bool, Infallible> {
        Ok(self.memory.balances.contains_key(key))
    }
}

impl StorageMutate<ContractsAssets> for MemoryStorage {
    fn replace(
        &mut self,
        key: &<ContractsAssets as Mappable>::Key,
        value: &Word,
    ) -> Result<Option<Word>, Infallible> {
        Ok(self.memory.balances.insert(*key, *value))
    }

    fn take(
        &mut self,
        key: &<ContractsAssets as Mappable>::Key,
    ) -> Result<Option<Word>, Infallible> {
        Ok(self.memory.balances.remove(key))
    }

    fn replace_forced(
        &mut self,
        _key: &<ContractsAssets as Mappable>::Key,
        _value: &<ContractsAssets as Mappable>::Value,
    ) -> Result<Option<<ContractsAssets as Mappable>::OwnedValue>, Self::Error> {
        unimplemented!()
    }
}

impl StorageInspect<ContractsState> for MemoryStorage {
    type Error = Infallible;

    fn get(
        &self,
        key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<Cow<'_, <ContractsState as Mappable>::OwnedValue>>, Infallible>
    {
        Ok(self.memory.contract_state.get(key).map(Cow::Borrowed))
    }

    fn contains_key(
        &self,
        key: &<ContractsState as Mappable>::Key,
    ) -> Result<bool, Infallible> {
        Ok(self.memory.contract_state.contains_key(key))
    }
}

impl StorageMutate<ContractsState> for MemoryStorage {
    fn replace(
        &mut self,
        key: &<ContractsState as Mappable>::Key,
        value: &<ContractsState as Mappable>::Value,
    ) -> Result<Option<<ContractsState as Mappable>::OwnedValue>, Infallible> {
        Ok(self.memory.contract_state.insert(*key, value.into()))
    }

    fn take(
        &mut self,
        key: &<ContractsState as Mappable>::Key,
    ) -> Result<Option<ContractsStateData>, Infallible> {
        Ok(self.memory.contract_state.remove(key))
    }

    fn replace_forced(
        &mut self,
        _key: &<ContractsState as Mappable>::Key,
        _value: &<ContractsState as Mappable>::Value,
    ) -> Result<Option<<ContractsState as Mappable>::OwnedValue>, Self::Error> {
        unimplemented!()
    }
}

impl StorageWrite<ContractsState> for MemoryStorage {
    fn write_bytes(
        &mut self,
        key: &<ContractsState as Mappable>::Key,
        buf: &[u8],
    ) -> Result<usize, Infallible> {
        let size = buf.len();
        self.memory
            .contract_state
            .insert(*key, ContractsStateData::from(buf));
        Ok(size)
    }

    fn replace_bytes(
        &mut self,
        key: &<ContractsState as Mappable>::Key,
        buf: &[u8],
    ) -> Result<(usize, Option<Vec<u8>>), Self::Error>
    where
        Self: StorageSize<ContractsState>,
    {
        let size = buf.len();
        let prev = self
            .memory
            .contract_state
            .insert(*key, ContractsStateData::from(buf))
            .map(Into::into);
        Ok((size, prev))
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
    ) -> Result<Option<usize>, Infallible> {
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
        buf: &mut [u8],
    ) -> Result<Option<usize>, Self::Error> {
        Ok(self.memory.contract_state.get(key).map(|data| {
            let len = buf.len().min(data.as_ref().len());
            buf.copy_from_slice(&data.as_ref()[..len]);
            len
        }))
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
    ) -> Result<Option<usize>, Infallible> {
        Ok(self.memory.blobs.get(key).map(|c| c.as_ref().len()))
    }
}

impl StorageRead<BlobData> for MemoryStorage {
    fn read(
        &self,
        key: &<BlobData as Mappable>::Key,
        buf: &mut [u8],
    ) -> Result<Option<usize>, Self::Error> {
        Ok(self.memory.blobs.get(key).map(|data| {
            let len = buf.len().min(data.as_ref().len());
            buf.copy_from_slice(&data.as_ref()[..len]);
            len
        }))
    }

    fn read_alloc(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.memory.blobs.get(key).map(|c| c.as_ref().to_vec()))
    }
}

impl StorageInspect<BlobData> for MemoryStorage {
    type Error = Infallible;

    fn get(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Cow<'_, <BlobData as Mappable>::OwnedValue>>, Infallible> {
        Ok(self.memory.blobs.get(key).map(Cow::Borrowed))
    }

    fn contains_key(
        &self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<bool, Infallible> {
        Ok(self.memory.blobs.contains_key(key))
    }
}

impl StorageMutate<BlobData> for MemoryStorage {
    fn replace(
        &mut self,
        key: &<BlobData as Mappable>::Key,
        value: &<BlobData as Mappable>::Value,
    ) -> Result<Option<<BlobData as Mappable>::OwnedValue>, Infallible> {
        Ok(self.memory.blobs.insert(*key, value.into()))
    }

    fn take(
        &mut self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<BlobBytes>, Infallible> {
        Ok(self.memory.blobs.remove(key))
    }

    fn replace_forced(
        &mut self,
        _key: &<BlobData as Mappable>::Key,
        _value: &<BlobData as Mappable>::Value,
    ) -> Result<Option<<BlobData as Mappable>::OwnedValue>, Self::Error> {
        unimplemented!()
    }
}

impl StorageWrite<BlobData> for MemoryStorage {
    fn write_bytes(
        &mut self,
        key: &<BlobData as Mappable>::Key,
        buf: &[u8],
    ) -> Result<usize, Infallible> {
        let size = buf.len();
        self.memory.blobs.insert(*key, BlobBytes::from(buf));
        Ok(size)
    }

    fn replace_bytes(
        &mut self,
        key: &<BlobData as Mappable>::Key,
        buf: &[u8],
    ) -> Result<(usize, Option<Vec<u8>>), Self::Error>
    where
        Self: StorageSize<BlobData>,
    {
        let size = buf.len();
        let prev = self
            .memory
            .blobs
            .insert(*key, BlobBytes::from(buf))
            .map(Into::into);
        Ok((size, prev))
    }

    fn take_bytes(
        &mut self,
        key: &<BlobData as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        let prev = self.memory.blobs.remove(key).map(Into::into);
        Ok(prev)
    }
}

impl ContractsAssetsStorage for MemoryStorage {}

impl InterpreterStorage for MemoryStorage {
    type DataError = Infallible;

    fn block_height(&self) -> Result<BlockHeight, Infallible> {
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

    fn block_hash(&self, block_height: BlockHeight) -> Result<Bytes32, Infallible> {
        Ok(Hasher::hash(block_height.to_be_bytes()))
    }

    fn coinbase(&self) -> Result<ContractId, Infallible> {
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
    ) -> Result<Vec<Option<Cow<ContractsStateData>>>, Self::DataError> {
        let start: ContractsStateKey = (id, start_key).into();
        let end: ContractsStateKey = (id, &Bytes32::new([u8::MAX; 32])).into();
        let mut iter = self.memory.contract_state.range(start..end);

        let mut next_item = iter.next();
        Ok(core::iter::successors(Some(**start_key), |n| {
            let mut n = *n;
            if add_one(&mut n) {
                None
            } else {
                Some(n)
            }
        })
        .map(|next_key: [u8; 32]| match next_item.take() {
            Some((k, v)) => match next_key.cmp(k.state_key()) {
                core::cmp::Ordering::Less => {
                    next_item = Some((k, v));
                    None
                }
                core::cmp::Ordering::Equal => {
                    next_item = iter.next();
                    Some(Cow::Borrowed(v))
                }
                core::cmp::Ordering::Greater => None,
            },
            None => None,
        })
        .take(range)
        .collect())
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
        let storage: &mut dyn StorageWrite<ContractsState, Error = Self::DataError> =
            self;
        let mut unset_count = 0;
        core::iter::successors(Some(**start_key), |n| {
            let mut n = *n;
            if add_one(&mut n) {
                None
            } else {
                Some(n)
            }
        })
        .zip(values)
        .try_for_each(|(key, value)| {
            let key: ContractsStateKey = (contract, &Bytes32::from(key)).into();
            // Safety: we never have over usize::MAX items in one call
            #[allow(clippy::arithmetic_side_effects)]
            if !storage.contains_key(&key)? {
                unset_count += 1;
            }
            storage.write_bytes(&key, value)?;
            Ok::<_, Self::DataError>(())
        })?;
        Ok(unset_count)
    }

    fn contract_state_remove_range(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        range: usize,
    ) -> Result<Option<()>, Self::DataError> {
        let mut all_set_key = true;
        let mut values: hashbrown::HashSet<_> =
            core::iter::successors(Some(**start_key), |n| {
                let mut n = *n;
                if add_one(&mut n) {
                    None
                } else {
                    Some(n)
                }
            })
            .take(range)
            .collect();
        self.memory.contract_state.retain(|key, _| {
            let c = key.contract_id();
            let k = key.state_key();
            let r = values.remove(&**k);
            all_set_key &= c == contract && r;
            c != contract || !r
        });
        Ok((all_set_key && values.is_empty()).then_some(()))
    }
}

impl PredicateStorageRequirements for MemoryStorage {
    fn storage_error_to_string(error: Self::Error) -> alloc::string::String {
        alloc::format!("{:?}", error)
    }
}

fn add_one(a: &mut [u8; 32]) -> bool {
    let right = u128::from_be_bytes(a[16..].try_into().unwrap());
    let (right, of) = right.overflowing_add(1);
    a[16..].copy_from_slice(&right.to_be_bytes()[..]);
    if of {
        let left = u128::from_be_bytes(a[..16].try_into().unwrap());
        let (left, of) = left.overflowing_add(1);
        a[..16].copy_from_slice(&left.to_be_bytes()[..]);
        return of
    }
    false
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
}

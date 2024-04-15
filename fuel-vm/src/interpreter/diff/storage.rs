use core::fmt::Debug;
use hashbrown::HashMap;

use fuel_storage::{
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

use crate::storage::{
    ContractsAssetKey,
    ContractsAssetsStorage,
    ContractsStateData,
    ContractsStateKey,
    InterpreterStorage,
    UploadedBytecode,
    UploadedBytecodes,
};

use super::{
    ExecutableTransaction,
    Interpreter,
    *,
};

#[derive(Debug)]
/// The set of state changes that are recorded.
pub(super) enum StorageDelta {
    State(MappableDelta<ContractsStateKey, ContractsStateData>),
    Assets(MappableDelta<ContractsAssetKey, u64>),
    RawCode(MappableDelta<ContractId, Contract>),
    UploadedBytecode(MappableDelta<Bytes32, UploadedBytecode>),
}

/// The set of states that are recorded.
#[derive(Debug, Clone)]
pub(super) enum StorageState {
    State(MappableState<ContractsStateKey, ContractsStateData>),
    Assets(MappableState<ContractsAssetKey, u64>),
    RawCode(MappableState<ContractId, Contract>),
    UploadedBytecode(MappableState<Bytes32, UploadedBytecode>),
}

#[derive(Debug)]
/// A [`Mappable`] type that has changed.
pub(super) enum MappableDelta<Key, Value> {
    Insert(Key, Value, Option<Value>),
    Remove(Key, Value),
}

/// The state of a [`Mappable`] type.
#[derive(Debug, Clone)]
pub(super) struct MappableState<Key, Value> {
    pub key: Key,
    pub value: Option<Value>,
}

/// Records state changes of any [`Mappable`] type.
pub(super) trait StorageType: Mappable {
    /// Records an insert state change.
    fn record_insert(
        key: &Self::Key,
        value: &Self::Value,
        existing: Option<Self::OwnedValue>,
    ) -> StorageDelta;

    /// Records a remove state change.
    fn record_remove(key: &Self::Key, value: Self::OwnedValue) -> StorageDelta;
}

#[derive(Debug)]
pub struct Record<S>(pub(super) S, pub(super) Vec<StorageDelta>)
where
    S: InterpreterStorage;

impl<S, Tx, Ecal> Interpreter<Record<S>, Tx, Ecal>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    /// Remove the [`Recording`] wrapper from the storage.
    /// Recording storage changes has an overhead so it's
    /// useful to be able to remove it once the diff is generated.
    pub fn remove_recording(self) -> Interpreter<S, Tx, Ecal> {
        Interpreter {
            registers: self.registers,
            memory: self.memory,
            frames: self.frames,
            receipts: self.receipts,
            tx: self.tx,
            initial_balances: self.initial_balances,
            storage: self.storage.0,
            debugger: self.debugger,
            context: self.context,
            balances: self.balances,
            panic_context: self.panic_context,
            profiler: self.profiler,
            interpreter_params: self.interpreter_params,
            ecal_state: self.ecal_state,
        }
    }

    /// Get the diff of changes to this VMs storage.
    pub fn storage_diff(&self) -> Diff<Deltas> {
        let mut diff = Diff {
            changes: Vec::new(),
        };
        let mut contracts_state = Delta {
            from: HashMap::new(),
            to: HashMap::new(),
        };
        let mut contracts_assets = Delta {
            from: HashMap::new(),
            to: HashMap::new(),
        };
        let mut contracts_raw_code = Delta {
            from: HashMap::new(),
            to: HashMap::new(),
        };
        let mut uploaded_bytecode = Delta {
            from: HashMap::new(),
            to: HashMap::new(),
        };

        for delta in self.storage.1.iter() {
            match delta {
                StorageDelta::State(delta) => {
                    mappable_delta_to_hashmap(&mut contracts_state, delta)
                }
                StorageDelta::Assets(delta) => {
                    mappable_delta_to_hashmap(&mut contracts_assets, delta)
                }
                StorageDelta::RawCode(delta) => {
                    mappable_delta_to_hashmap(&mut contracts_raw_code, delta)
                }
                StorageDelta::UploadedBytecode(delta) => {
                    mappable_delta_to_hashmap(&mut uploaded_bytecode, delta)
                }
            }
        }
        storage_state_to_changes(&mut diff, contracts_state, StorageState::State);
        storage_state_to_changes(&mut diff, contracts_assets, StorageState::Assets);
        storage_state_to_changes(&mut diff, contracts_raw_code, StorageState::RawCode);
        storage_state_to_changes(
            &mut diff,
            uploaded_bytecode,
            StorageState::UploadedBytecode,
        );
        diff
    }
}

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    /// Add a [`Recording`] wrapper around the storage to
    /// record any changes this VM makes to it's storage.
    /// Recording storage changes has an overhead so should
    /// be used in production.
    pub fn add_recording(self) -> Interpreter<Record<S>, Tx, Ecal> {
        Interpreter {
            registers: self.registers,
            memory: self.memory,
            frames: self.frames,
            receipts: self.receipts,
            tx: self.tx,
            initial_balances: self.initial_balances,
            storage: Record::new(self.storage),
            debugger: self.debugger,
            context: self.context,
            balances: self.balances,
            panic_context: self.panic_context,
            profiler: self.profiler,
            interpreter_params: self.interpreter_params,
            ecal_state: self.ecal_state,
        }
    }

    /// Change this VMs internal state to match the initial state from this diff.
    pub fn reset_vm_state(&mut self, diff: &Diff<InitialVmState>)
    where
        Tx: Clone + 'static,
    {
        for change in &diff.changes {
            self.inverse_inner(change);
            if let Change::Storage(Previous(from)) = change {
                match from {
                    StorageState::State(MappableState { key, value }) => {
                        if let Some(value) = value {
                            StorageMutate::<ContractsState>::insert(
                                &mut self.storage,
                                key,
                                value.as_ref(),
                            )
                            .unwrap();
                        }
                    }
                    StorageState::Assets(MappableState { key, value }) => {
                        if let Some(value) = value {
                            StorageMutate::<ContractsAssets>::insert(
                                &mut self.storage,
                                key,
                                value,
                            )
                            .unwrap();
                        }
                    }
                    StorageState::RawCode(MappableState { key, value }) => {
                        if let Some(value) = value {
                            StorageMutate::<ContractsRawCode>::insert(
                                &mut self.storage,
                                key,
                                value.as_ref(),
                            )
                            .unwrap();
                        }
                    }
                    StorageState::UploadedBytecode(MappableState { key, value }) => {
                        if let Some(value) = value {
                            StorageMutate::<UploadedBytecodes>::insert(
                                &mut self.storage,
                                key,
                                value,
                            )
                            .unwrap();
                        }
                    }
                }
            }
        }
    }
}

fn mappable_delta_to_hashmap<'value, K, V>(
    state: &mut Delta<HashMap<K, &'value V>>,
    delta: &'value MappableDelta<K, V>,
) where
    K: Copy + PartialEq + Eq + core::hash::Hash + 'static,
    V: Clone + 'static,
{
    match delta {
        MappableDelta::Insert(key, value, Some(existing)) => {
            state.from.entry(*key).or_insert(existing);
            state.to.insert(*key, value);
        }
        MappableDelta::Insert(key, value, None) => {
            state.to.insert(*key, value);
        }
        MappableDelta::Remove(key, existing) => {
            state.from.entry(*key).or_insert(existing);
            state.to.remove(key);
        }
    }
}

fn storage_state_to_changes<K, V>(
    diff: &mut Diff<Deltas>,
    state: Delta<HashMap<K, &V>>,
    f: fn(MappableState<K, V>) -> StorageState,
) where
    K: Copy + PartialEq + Eq + Hash + 'static,
    V: Clone + 'static,
{
    let Delta { mut from, to } = state;
    let iter = to.into_iter().map(|(k, v)| {
        Change::Storage(Delta {
            from: f(MappableState {
                key: k,
                value: from.remove(&k).cloned(),
            }),
            to: f(MappableState {
                key: k,
                value: Some(v.clone()),
            }),
        })
    });
    diff.changes.extend(iter);
    let iter = from.into_iter().map(|(k, v)| {
        Change::Storage(Delta {
            from: f(MappableState {
                key: k,
                value: Some(v.clone()),
            }),
            to: f(MappableState {
                key: k,
                value: None,
            }),
        })
    });
    diff.changes.extend(iter);
}

impl<Type: Mappable, S> StorageInspect<Type> for Record<S>
where
    S: StorageInspect<Type>,
    S: InterpreterStorage,
{
    type Error = <S as StorageInspect<Type>>::Error;

    fn get(
        &self,
        key: &<Type as Mappable>::Key,
    ) -> Result<Option<alloc::borrow::Cow<<Type as Mappable>::OwnedValue>>, Self::Error>
    {
        <S as StorageInspect<Type>>::get(&self.0, key)
    }

    fn contains_key(&self, key: &<Type as Mappable>::Key) -> Result<bool, Self::Error> {
        <S as StorageInspect<Type>>::contains_key(&self.0, key)
    }
}

impl<Type: Mappable, S> StorageSize<Type> for Record<S>
where
    S: StorageSize<Type>,
    S: InterpreterStorage,
{
    fn size_of_value(
        &self,
        key: &<Type as Mappable>::Key,
    ) -> Result<Option<usize>, Self::Error> {
        <S as StorageSize<Type>>::size_of_value(&self.0, key)
    }
}

impl<Type: Mappable, S> StorageRead<Type> for Record<S>
where
    S: StorageRead<Type>,
    S: InterpreterStorage,
{
    fn read(
        &self,
        key: &<Type as Mappable>::Key,
        buf: &mut [u8],
    ) -> Result<Option<usize>, Self::Error> {
        <S as StorageRead<Type>>::read(&self.0, key, buf)
    }

    fn read_alloc(
        &self,
        key: &<Type as Mappable>::Key,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        <S as StorageRead<Type>>::read_alloc(&self.0, key)
    }
}

impl<Type: StorageType, S> StorageMutate<Type> for Record<S>
where
    S: StorageInspect<Type>,
    S: StorageMutate<Type>,
    S: InterpreterStorage,
{
    fn insert(
        &mut self,
        key: &<Type as Mappable>::Key,
        value: &<Type as Mappable>::Value,
    ) -> Result<Option<<Type as Mappable>::OwnedValue>, Self::Error> {
        let existing = <S as StorageMutate<Type>>::insert(&mut self.0, key, value)?;
        self.1.push(<Type as StorageType>::record_insert(
            key,
            value,
            existing.clone(),
        ));
        Ok(existing)
    }

    fn remove(
        &mut self,
        key: &<Type as Mappable>::Key,
    ) -> Result<Option<<Type as Mappable>::OwnedValue>, Self::Error> {
        let existing = <S as StorageMutate<Type>>::remove(&mut self.0, key)?;
        if let Some(existing) = &existing {
            self.1
                .push(<Type as StorageType>::record_remove(key, existing.clone()));
        }
        Ok(existing)
    }
}

impl<Type: StorageType, S> StorageWrite<Type> for Record<S>
where
    S: StorageWrite<Type>,
    S: InterpreterStorage,
{
    fn write(&mut self, key: &Type::Key, buf: &[u8]) -> Result<usize, Self::Error> {
        <S as StorageWrite<Type>>::write(&mut self.0, key, buf)
    }

    fn replace(
        &mut self,
        key: &Type::Key,
        buf: &[u8],
    ) -> Result<(usize, Option<Vec<u8>>), Self::Error> {
        <S as StorageWrite<Type>>::replace(&mut self.0, key, buf)
    }

    fn take(&mut self, key: &Type::Key) -> Result<Option<Vec<u8>>, Self::Error> {
        <S as StorageWrite<Type>>::take(&mut self.0, key)
    }
}

impl<S: ContractsAssetsStorage + InterpreterStorage> ContractsAssetsStorage
    for Record<S>
{
}

impl<S> InterpreterStorage for Record<S>
where
    S: InterpreterStorage,
{
    type DataError = <S as InterpreterStorage>::DataError;

    fn block_height(&self) -> Result<BlockHeight, Self::DataError> {
        self.0.block_height()
    }

    fn consensus_parameters_version(&self) -> Result<u32, Self::DataError> {
        self.0.consensus_parameters_version()
    }

    fn state_transition_version(&self) -> Result<u32, Self::DataError> {
        self.0.state_transition_version()
    }

    fn timestamp(&self, height: BlockHeight) -> Result<Word, Self::DataError> {
        self.0.timestamp(height)
    }

    fn block_hash(&self, block_height: BlockHeight) -> Result<Bytes32, Self::DataError> {
        self.0.block_hash(block_height)
    }

    fn coinbase(&self) -> Result<fuel_types::ContractId, Self::DataError> {
        self.0.coinbase()
    }

    fn set_consensus_parameters(
        &mut self,
        version: u32,
        consensus_parameters: &ConsensusParameters,
    ) -> Result<Option<ConsensusParameters>, Self::DataError> {
        self.0
            .set_consensus_parameters(version, consensus_parameters)
    }

    fn set_state_transition_bytecode(
        &mut self,
        version: u32,
        hash: &Bytes32,
    ) -> Result<Option<Bytes32>, Self::DataError> {
        self.0.set_state_transition_bytecode(version, hash)
    }

    fn contract_state_range(
        &self,
        id: &ContractId,
        start_key: &Bytes32,
        range: usize,
    ) -> Result<Vec<Option<alloc::borrow::Cow<ContractsStateData>>>, Self::DataError>
    {
        self.0.contract_state_range(id, start_key, range)
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
        self.0
            .contract_state_insert_range(contract, start_key, values)
    }

    fn contract_state_remove_range(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        range: usize,
    ) -> Result<Option<()>, S::DataError> {
        self.0
            .contract_state_remove_range(contract, start_key, range)
    }
}

impl StorageType for ContractsState {
    fn record_insert(
        key: &Self::Key,
        value: &[u8],
        existing: Option<ContractsStateData>,
    ) -> StorageDelta {
        StorageDelta::State(MappableDelta::Insert(*key, value.into(), existing))
    }

    fn record_remove(key: &Self::Key, value: ContractsStateData) -> StorageDelta {
        StorageDelta::State(MappableDelta::Remove(*key, value))
    }
}

impl StorageType for ContractsAssets {
    fn record_insert(
        key: &Self::Key,
        value: &u64,
        existing: Option<u64>,
    ) -> StorageDelta {
        StorageDelta::Assets(MappableDelta::Insert(*key, *value, existing))
    }

    fn record_remove(key: &Self::Key, value: u64) -> StorageDelta {
        StorageDelta::Assets(MappableDelta::Remove(*key, value))
    }
}

impl StorageType for ContractsRawCode {
    fn record_insert(
        key: &ContractId,
        value: &[u8],
        existing: Option<Contract>,
    ) -> StorageDelta {
        StorageDelta::RawCode(MappableDelta::Insert(*key, value.into(), existing))
    }

    fn record_remove(key: &ContractId, value: Contract) -> StorageDelta {
        StorageDelta::RawCode(MappableDelta::Remove(*key, value))
    }
}

impl StorageType for UploadedBytecodes {
    fn record_insert(
        key: &Bytes32,
        value: &UploadedBytecode,
        existing: Option<UploadedBytecode>,
    ) -> StorageDelta {
        StorageDelta::UploadedBytecode(MappableDelta::Insert(
            *key,
            value.clone(),
            existing,
        ))
    }

    fn record_remove(key: &Bytes32, value: UploadedBytecode) -> StorageDelta {
        StorageDelta::UploadedBytecode(MappableDelta::Remove(*key, value))
    }
}

impl<S> Record<S>
where
    S: InterpreterStorage,
{
    pub fn new(s: S) -> Self {
        Self(s, Vec::new())
    }
}

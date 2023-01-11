use std::collections::HashMap;
use std::fmt::Debug;

use fuel_types::AssetId;
use fuel_types::Bytes32;
use fuel_types::ContractId;

use crate::storage::InterpreterStorage;

use super::ExecutableTransaction;
use super::Interpreter;
use super::*;

#[derive(Debug)]
/// The set of state changes that are recorded.
pub(super) enum StorageDelta {
    State(MappableDelta<(ContractId, Bytes32), Bytes32>),
    Assets(MappableDelta<(ContractId, AssetId), u64>),
    Info(MappableDelta<ContractId, (fuel_types::Salt, Bytes32)>),
    RawCode(MappableDelta<ContractId, Vec<u8>>),
}

/// The set of states that are recorded.
#[derive(Debug, Clone)]
pub(super) enum StorageState {
    State(MappableState<(ContractId, Bytes32), Bytes32>),
    Assets(MappableState<(ContractId, AssetId), u64>),
    Info(MappableState<ContractId, (fuel_types::Salt, Bytes32)>),
    RawCode(MappableState<ContractId, Vec<u8>>),
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
pub(super) trait StorageType<Key, SetValue, GetValue>
where
    Key: ToOwnedKey,
    SetValue: ToOwned + ?Sized,
    GetValue: ToSetValue<<SetValue as ToOwned>::Owned>,
{
    /// Records an insert state change.
    fn record_insert(key: &Key, value: &SetValue, existing: Option<GetValue>) -> StorageDelta;

    /// Records a remove state change.
    fn record_remove(key: &Key, value: GetValue) -> StorageDelta;

    fn to_insert(
        key: &Key,
        value: &SetValue,
        existing: Option<GetValue>,
    ) -> MappableDelta<<Key as ToOwnedKey>::OwnedKey, <SetValue as ToOwned>::Owned> {
        MappableDelta::Insert(key.to_owned_key(), value.to_owned(), existing.map(|v| v.to_set_value()))
    }

    fn to_remove(
        key: &Key,
        value: GetValue,
    ) -> MappableDelta<<Key as ToOwnedKey>::OwnedKey, <SetValue as ToOwned>::Owned> {
        MappableDelta::Remove(key.to_owned_key(), value.to_set_value())
    }
}

pub(super) trait ToOwnedKey {
    type OwnedKey;
    fn to_owned_key(&self) -> Self::OwnedKey;
}

pub(super) trait ToSetValue<T> {
    fn to_set_value(&self) -> T;
}

impl<T, U> ToSetValue<T> for U
where
    U: ToOwned,
    <U as ToOwned>::Owned: Into<T>,
{
    fn to_set_value(&self) -> T {
        self.to_owned().into()
    }
}

#[derive(Debug)]
pub struct Record<S>(pub(super) S, pub(super) Vec<StorageDelta>)
where
    S: InterpreterStorage;

impl<S, Tx> Interpreter<Record<S>, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    /// Remove the [`Recording`] wrapper from the storage.
    /// Recording storage changes has an overhead so it's
    /// useful to be able to remove it once the diff is generated.
    pub fn remove_recording(self) -> Interpreter<S, Tx> {
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
            gas_costs: self.gas_costs,
            params: self.params,
            panic_context: self.panic_context,
            #[cfg(feature = "profile-any")]
            profiler: self.profiler,
        }
    }

    /// Get the diff of changes to this VMs storage.
    pub fn storage_diff(&self) -> Diff<Deltas> {
        let mut diff = Diff { changes: Vec::new() };
        let mut contracts_state = Delta {
            from: HashMap::new(),
            to: HashMap::new(),
        };
        let mut contracts_assets = Delta {
            from: HashMap::new(),
            to: HashMap::new(),
        };
        let mut contracts_info = Delta {
            from: HashMap::new(),
            to: HashMap::new(),
        };
        let mut contracts_raw_code = Delta {
            from: HashMap::new(),
            to: HashMap::new(),
        };

        for delta in self.storage.1.iter() {
            match delta {
                StorageDelta::State(delta) => mappable_delta_to_hashmap(&mut contracts_state, delta),
                StorageDelta::Assets(delta) => mappable_delta_to_hashmap(&mut contracts_assets, delta),
                StorageDelta::Info(delta) => mappable_delta_to_hashmap(&mut contracts_info, delta),
                StorageDelta::RawCode(delta) => mappable_delta_to_hashmap(&mut contracts_raw_code, delta),
            }
        }
        storage_state_to_changes(&mut diff, contracts_state, StorageState::State);
        storage_state_to_changes(&mut diff, contracts_info, StorageState::Info);
        storage_state_to_changes(&mut diff, contracts_assets, StorageState::Assets);
        storage_state_to_changes(&mut diff, contracts_raw_code, StorageState::RawCode);
        diff
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    /// Add a [`Recording`] wrapper around the storage to
    /// record any changes this VM makes to it's storage.
    /// Recording storage changes has an overhead so should
    /// be used in production.
    pub fn add_recording(self) -> Interpreter<Record<S>, Tx> {
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
            gas_costs: self.gas_costs,
            params: self.params,
            panic_context: self.panic_context,
            #[cfg(feature = "profile-any")]
            profiler: self.profiler,
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
                            StorageMutate::<ContractsState>::insert(&mut self.storage, &(&key.0, &key.1), value)
                                .unwrap();
                        }
                    }
                    StorageState::Assets(MappableState { key, value }) => {
                        if let Some(value) = value {
                            StorageMutate::<ContractsAssets>::insert(&mut self.storage, &(&key.0, &key.1), value)
                                .unwrap();
                        }
                    }
                    StorageState::Info(MappableState { key, value }) => {
                        if let Some(value) = value {
                            StorageMutate::<ContractsInfo>::insert(&mut self.storage, key, value).unwrap();
                        }
                    }
                    StorageState::RawCode(MappableState { key, value }) => {
                        if let Some(value) = value {
                            StorageMutate::<ContractsRawCode>::insert(&mut self.storage, key, value).unwrap();
                        }
                    }
                }
            }
        }
    }
}

fn mappable_delta_to_hashmap<'value, K, V>(state: &mut Delta<HashMap<K, &'value V>>, delta: &'value MappableDelta<K, V>)
where
    K: Copy + PartialEq + Eq + std::hash::Hash + 'static,
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
    K: Copy + PartialEq + Eq + std::hash::Hash + 'static,
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
            to: f(MappableState { key: k, value: None }),
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
    ) -> Result<Option<std::borrow::Cow<<Type as Mappable>::GetValue>>, Self::Error> {
        <S as StorageInspect<Type>>::get(&self.0, key)
    }

    fn contains_key(&self, key: &<Type as Mappable>::Key) -> Result<bool, Self::Error> {
        <S as StorageInspect<Type>>::contains_key(&self.0, key)
    }
}

impl<Type: Mappable, S> StorageMutate<Type> for Record<S>
where
    S: InterpreterStorage,
    S: StorageInspect<Type>,
    S: StorageMutate<Type>,
    <Type as Mappable>::Key: ToOwnedKey,
    <Type as Mappable>::SetValue: ToOwned,
    <Type as Mappable>::GetValue: ToSetValue<<<Type as Mappable>::SetValue as ToOwned>::Owned>,
    Type: StorageType<<Type as Mappable>::Key, <Type as Mappable>::SetValue, <Type as Mappable>::GetValue>,
{
    fn insert(
        &mut self,
        key: &<Type as Mappable>::Key,
        value: &<Type as Mappable>::SetValue,
    ) -> Result<Option<<Type as Mappable>::GetValue>, Self::Error> {
        let existing = <S as StorageMutate<Type>>::insert(&mut self.0, key, value)?;
        self.1.push(<Type as StorageType<
            <Type as Mappable>::Key,
            <Type as Mappable>::SetValue,
            <Type as Mappable>::GetValue,
        >>::record_insert(key, value, existing.clone()));
        Ok(existing)
    }

    fn remove(&mut self, key: &<Type as Mappable>::Key) -> Result<Option<<Type as Mappable>::GetValue>, Self::Error> {
        let existing = <S as StorageMutate<Type>>::remove(&mut self.0, key)?;
        if let Some(existing) = &existing {
            self.1.push(<Type as StorageType<
                <Type as Mappable>::Key,
                <Type as Mappable>::SetValue,
                <Type as Mappable>::GetValue,
            >>::record_remove(key, existing.clone()));
        }
        Ok(existing)
    }
}

impl<Key, Type: Mappable, S> MerkleRootStorage<Key, Type> for Record<S>
where
    S: InterpreterStorage,
    S: MerkleRootStorage<Key, Type>,
    <Type as Mappable>::Key: ToOwnedKey,
    <Type as Mappable>::SetValue: ToOwned,
    <Type as Mappable>::GetValue: ToSetValue<<<Type as Mappable>::SetValue as ToOwned>::Owned>,
    Type: StorageType<<Type as Mappable>::Key, <Type as Mappable>::SetValue, <Type as Mappable>::GetValue>,
{
    fn root(&mut self, key: &Key) -> Result<fuel_storage::MerkleRoot, Self::Error> {
        <S as MerkleRootStorage<Key, Type>>::root(&mut self.0, key)
    }
}
impl<S> InterpreterStorage for Record<S>
where
    S: InterpreterStorage,
{
    type DataError = <S as InterpreterStorage>::DataError;

    fn block_height(&self) -> Result<u32, Self::DataError> {
        self.0.block_height()
    }

    fn timestamp(&self, height: u32) -> Result<Word, Self::DataError> {
        self.0.timestamp(height)
    }

    fn block_hash(&self, block_height: u32) -> Result<fuel_types::Bytes32, Self::DataError> {
        self.0.block_hash(block_height)
    }

    fn coinbase(&self) -> Result<fuel_types::Address, Self::DataError> {
        self.0.coinbase()
    }

    fn merkle_contract_state_range(
        &self,
        id: &fuel_types::ContractId,
        start_key: &fuel_types::Bytes32,
        range: Word,
    ) -> Result<Vec<Option<std::borrow::Cow<fuel_types::Bytes32>>>, Self::DataError> {
        self.0.merkle_contract_state_range(id, start_key, range)
    }

    fn merkle_contract_state_insert_range(
        &mut self,
        contract: &fuel_types::ContractId,
        start_key: &fuel_types::Bytes32,
        values: &[fuel_types::Bytes32],
    ) -> Result<Option<()>, Self::DataError> {
        self.0.merkle_contract_state_insert_range(contract, start_key, values)
    }

    fn merkle_contract_state_remove_range(
        &mut self,
        contract: &fuel_types::ContractId,
        start_key: &fuel_types::Bytes32,
        range: Word,
    ) -> Result<Option<()>, Self::DataError> {
        self.0.merkle_contract_state_remove_range(contract, start_key, range)
    }
}

impl ToOwnedKey for (&ContractId, &Bytes32) {
    type OwnedKey = (ContractId, Bytes32);

    fn to_owned_key(&self) -> Self::OwnedKey {
        (*self.0, *self.1)
    }
}

impl ToOwnedKey for ContractId {
    type OwnedKey = ContractId;

    fn to_owned_key(&self) -> Self::OwnedKey {
        *self
    }
}

impl ToOwnedKey for (&ContractId, &AssetId) {
    type OwnedKey = (ContractId, AssetId);

    fn to_owned_key(&self) -> Self::OwnedKey {
        (*self.0, *self.1)
    }
}

impl StorageType<(&ContractId, &Bytes32), Bytes32, Bytes32> for ContractsState<'_> {
    fn record_insert(key: &(&ContractId, &Bytes32), value: &Bytes32, existing: Option<Bytes32>) -> StorageDelta {
        StorageDelta::State(Self::to_insert(key, value, existing))
    }

    fn record_remove(key: &(&ContractId, &Bytes32), value: Bytes32) -> StorageDelta {
        StorageDelta::State(Self::to_remove(key, value))
    }
}
impl StorageType<(&ContractId, &AssetId), u64, u64> for ContractsAssets<'_> {
    fn record_insert(key: &(&ContractId, &AssetId), value: &u64, existing: Option<u64>) -> StorageDelta {
        StorageDelta::Assets(Self::to_insert(key, value, existing))
    }

    fn record_remove(key: &(&ContractId, &AssetId), value: u64) -> StorageDelta {
        StorageDelta::Assets(Self::to_remove(key, value))
    }
}
impl StorageType<ContractId, (fuel_types::Salt, Bytes32), (fuel_types::Salt, Bytes32)> for ContractsInfo {
    fn record_insert(
        key: &ContractId,
        value: &(fuel_types::Salt, Bytes32),
        existing: Option<(fuel_types::Salt, Bytes32)>,
    ) -> StorageDelta {
        StorageDelta::Info(Self::to_insert(key, value, existing))
    }

    fn record_remove(key: &ContractId, value: (fuel_types::Salt, Bytes32)) -> StorageDelta {
        StorageDelta::Info(Self::to_remove(key, value))
    }
}
impl StorageType<ContractId, [u8], Contract> for ContractsRawCode {
    fn record_insert(key: &ContractId, value: &[u8], existing: Option<Contract>) -> StorageDelta {
        StorageDelta::RawCode(Self::to_insert(key, value, existing))
    }

    fn record_remove(key: &ContractId, value: Contract) -> StorageDelta {
        StorageDelta::RawCode(Self::to_remove(key, value))
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

use crate::crypto;
use crate::error::Infallible;
use crate::storage::{ContractsAssets, ContractsInfo, ContractsRawCode, ContractsState, InterpreterStorage};

use fuel_crypto::Hasher;
use fuel_storage::{MerkleRoot, MerkleRootStorage, StorageAsRef, StorageInspect, StorageMutate};
use fuel_tx::Contract;
use fuel_types::{Address, AssetId, Bytes32, ContractId, Salt, Word};
use itertools::Itertools;
use tai64::Tai64;
use tuples::TupleCloned;

use std::borrow::Cow;
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct MemoryStorageInner {
    contracts: BTreeMap<ContractId, Contract>,
    balances: BTreeMap<(ContractId, AssetId), Word>,
    contract_state: BTreeMap<(ContractId, Bytes32), Bytes32>,
    contract_code_root: BTreeMap<ContractId, (Salt, Bytes32)>,
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
    block_height: u32,
    coinbase: Address,
    memory: MemoryStorageInner,
    transacted: MemoryStorageInner,
    persisted: MemoryStorageInner,
}

impl MemoryStorage {
    /// Create a new memory storage.
    pub fn new(block_height: u32, coinbase: Address) -> Self {
        Self {
            block_height,
            coinbase,
            memory: Default::default(),
            transacted: Default::default(),
            persisted: Default::default(),
        }
    }

    /// Iterate over all contract state in storage
    pub fn all_contract_state(&self) -> impl Iterator<Item = (&(ContractId, Bytes32), &Bytes32)> {
        self.memory.contract_state.iter()
    }

    /// Fetch a mapping from the contract state.
    pub fn contract_state(&self, contract: &ContractId, key: &Bytes32) -> Cow<'_, Bytes32> {
        const DEFAULT_STATE: Bytes32 = Bytes32::zeroed();

        self.storage::<ContractsState>()
            .get(&(contract, key))
            .expect("Infallible")
            .unwrap_or(Cow::Borrowed(&DEFAULT_STATE))
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
    pub fn set_block_height(&mut self, block_height: u32) {
        self.block_height = block_height;
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        let block_height = 1;
        let coinbase = Address::from(*Hasher::hash(b"coinbase"));

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
    fn insert(&mut self, key: &ContractId, value: &[u8]) -> Result<Option<Contract>, Infallible> {
        Ok(self.memory.contracts.insert(*key, value.into()))
    }

    fn remove(&mut self, key: &ContractId) -> Result<Option<Contract>, Infallible> {
        Ok(self.memory.contracts.remove(key))
    }
}

impl StorageInspect<ContractsInfo> for MemoryStorage {
    type Error = Infallible;

    fn get(&self, key: &ContractId) -> Result<Option<Cow<'_, (Salt, Bytes32)>>, Infallible> {
        Ok(self.memory.contract_code_root.get(key).map(Cow::Borrowed))
    }

    fn contains_key(&self, key: &ContractId) -> Result<bool, Infallible> {
        Ok(self.memory.contract_code_root.contains_key(key))
    }
}

impl StorageMutate<ContractsInfo> for MemoryStorage {
    fn insert(&mut self, key: &ContractId, value: &(Salt, Bytes32)) -> Result<Option<(Salt, Bytes32)>, Infallible> {
        Ok(self.memory.contract_code_root.insert(*key, *value))
    }

    fn remove(&mut self, key: &ContractId) -> Result<Option<(Salt, Bytes32)>, Infallible> {
        Ok(self.memory.contract_code_root.remove(key))
    }
}

// TODO: Optimize `balances` to work with `&(&ContractId, &AssetId)` instead of `&(ContractId, AssetId)`
impl StorageInspect<ContractsAssets> for MemoryStorage {
    type Error = Infallible;

    fn get(&self, key: &(&ContractId, &AssetId)) -> Result<Option<Cow<'_, Word>>, Infallible> {
        Ok(self.memory.balances.get(&(*key).cloned()).copied().map(Cow::Owned))
    }

    fn contains_key(&self, key: &(&ContractId, &AssetId)) -> Result<bool, Infallible> {
        Ok(self.memory.balances.contains_key(&(*key).cloned()))
    }
}

impl StorageMutate<ContractsAssets> for MemoryStorage {
    fn insert(&mut self, key: &(&ContractId, &AssetId), value: &Word) -> Result<Option<Word>, Infallible> {
        Ok(self.memory.balances.insert((*key.0, *key.1), *value))
    }

    // TODO: Optimize `balances` to remove by `&(&ContractId, &AssetId)` instead of `&(ContractId, AssetId)`
    fn remove(&mut self, key: &(&ContractId, &AssetId)) -> Result<Option<Word>, Infallible> {
        Ok(self.memory.balances.remove(&(*key).cloned()))
    }
}

impl MerkleRootStorage<ContractId, ContractsAssets> for MemoryStorage {
    fn root(&mut self, parent: &ContractId) -> Result<MerkleRoot, Infallible> {
        let root = self
            .memory
            .balances
            .iter()
            .filter_map(|((contract, asset_id), balance)| (contract == parent).then_some((asset_id, balance)))
            .sorted_by_key(|t| t.0)
            .map(|(_, &balance)| balance)
            .map(Word::to_be_bytes);

        Ok(crypto::ephemeral_merkle_root(root).into())
    }
}

// TODO: Optimize `contract_state` to work with `&(&ContractId, &Bytes32)` instead of `&(ContractId, Bytes32)`
impl StorageInspect<ContractsState> for MemoryStorage {
    type Error = Infallible;

    fn get(&self, key: &(&ContractId, &Bytes32)) -> Result<Option<Cow<'_, Bytes32>>, Infallible> {
        Ok(self.memory.contract_state.get(&(*key).cloned()).map(Cow::Borrowed))
    }

    fn contains_key(&self, key: &(&ContractId, &Bytes32)) -> Result<bool, Infallible> {
        Ok(self.memory.contract_state.contains_key(&(*key).cloned()))
    }
}

impl StorageMutate<ContractsState> for MemoryStorage {
    fn insert(&mut self, key: &(&ContractId, &Bytes32), value: &Bytes32) -> Result<Option<Bytes32>, Infallible> {
        Ok(self.memory.contract_state.insert((*key.0, *key.1), *value))
    }

    // TODO: Optimize `contract_state` to remove by `&(&ContractId, &Bytes32)` instead of `&(ContractId, Bytes32)`
    fn remove(&mut self, key: &(&ContractId, &Bytes32)) -> Result<Option<Bytes32>, Infallible> {
        Ok(self.memory.contract_state.remove(&(*key).cloned()))
    }
}

impl MerkleRootStorage<ContractId, ContractsState> for MemoryStorage {
    fn root(&mut self, parent: &ContractId) -> Result<MerkleRoot, Infallible> {
        let root = self
            .memory
            .contract_state
            .iter()
            .filter_map(|((contract, key), value)| (contract == parent).then_some((key, value)))
            .sorted_by_key(|t| t.0)
            .map(|(_, value)| value);

        Ok(crypto::ephemeral_merkle_root(root).into())
    }
}

impl InterpreterStorage for MemoryStorage {
    type DataError = Infallible;

    fn block_height(&self) -> Result<u32, Infallible> {
        Ok(self.block_height)
    }

    fn timestamp(&self, height: u32) -> Result<Word, Self::DataError> {
        const GENESIS: Tai64 = Tai64::UNIX_EPOCH;
        const INTERVAL: Word = 10;

        Ok((GENESIS + (height as Word * INTERVAL)).0)
    }

    fn block_hash(&self, block_height: u32) -> Result<Bytes32, Infallible> {
        Ok(Hasher::hash(block_height.to_be_bytes()))
    }

    fn coinbase(&self) -> Result<Address, Infallible> {
        Ok(self.coinbase)
    }

    fn merkle_contract_state_range(
        &self,
        id: &ContractId,
        start_key: &Bytes32,
        range: Word,
    ) -> Result<Vec<Option<Cow<Bytes32>>>, Self::DataError> {
        let mut iter = self
            .memory
            .contract_state
            .range((*id, *start_key)..(*id, Bytes32::new([u8::MAX; 32])));

        let mut next_item = iter.next();
        Ok(std::iter::successors(Some(**start_key), |n| {
            let mut n = *n;
            if add_one(&mut n) {
                None
            } else {
                Some(n)
            }
        })
        .map(|next_key: [u8; 32]| match next_item.take() {
            Some((k, v)) => match next_key.cmp(&*k.1) {
                std::cmp::Ordering::Less => {
                    next_item = Some((k, v));
                    None
                }
                std::cmp::Ordering::Equal => {
                    next_item = iter.next();
                    Some(Cow::Borrowed(v))
                }
                std::cmp::Ordering::Greater => None,
            },
            None => None,
        })
        .take(range as usize)
        .collect())
    }

    fn merkle_contract_state_insert_range(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        values: &[Bytes32],
    ) -> Result<Option<()>, Self::DataError> {
        let mut any_unset_key = false;
        let values: Vec<_> = std::iter::successors(Some(**start_key), |n| {
            let mut n = *n;
            if add_one(&mut n) {
                None
            } else {
                Some(n)
            }
        })
        .zip(values)
        .map(|(key, value)| {
            let key = (*contract, Bytes32::from(key));
            any_unset_key |= !self.memory.contract_state.contains_key(&key);
            (key, *value)
        })
        .collect();
        self.memory.contract_state.extend(values);
        Ok((!any_unset_key).then_some(()))
    }

    fn merkle_contract_state_remove_range(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        range: Word,
    ) -> Result<Option<()>, Self::DataError> {
        let mut all_set_key = true;
        let mut values: std::collections::HashSet<_> = std::iter::successors(Some(**start_key), |n| {
            let mut n = *n;
            if add_one(&mut n) {
                None
            } else {
                Some(n)
            }
        })
        .take(range as usize)
        .collect();
        self.memory.contract_state.retain(|(c, k), _| {
            let r = values.remove(&**k);
            all_set_key &= c == contract && r;
            c != contract || !r
        });
        Ok((all_set_key && values.is_empty()).then_some(()))
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
        return of;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    const fn key(k: u8) -> [u8; 32] {
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, k,
        ]
    }

    #[test_case(&[&[0u8; 32]], &[0u8; 32], 1 => vec![Some(Bytes32::zeroed())])]
    #[test_case(&[&[0u8; 32]], &[0u8; 32], 0 => Vec::<Option<Bytes32>>::with_capacity(0))]
    #[test_case(&[], &[0u8; 32], 1 => vec![None])]
    #[test_case(&[], &[1u8; 32], 1 => vec![None])]
    #[test_case(&[&[0u8; 32]], &key(1), 2 => vec![None, None])]
    #[test_case(&[&key(1), &key(3)], &[0u8; 32], 4 => vec![None, Some(Bytes32::zeroed()), None, Some(Bytes32::zeroed())])]
    #[test_case(&[&[0u8; 32], &key(1)], &[0u8; 32], 1 => vec![Some(Bytes32::zeroed())])]
    fn test_contract_state_range(store: &[&[u8; 32]], start: &[u8; 32], range: Word) -> Vec<Option<Bytes32>> {
        let mut mem = MemoryStorage::default();
        for k in store {
            mem.memory
                .contract_state
                .insert((ContractId::default(), (**k).into()), Bytes32::zeroed());
        }
        mem.merkle_contract_state_range(&ContractId::default(), &(*start).into(), range)
            .unwrap()
            .into_iter()
            .map(|v| v.map(|v| v.into_owned()))
            .collect()
    }
}

use super::InterpreterStorage;
use crate::contract::Contract;
use crate::crypto;

use fuel_data::{Address, Bytes32, Color, ContractId, MerkleStorage, Salt, Storage, Word};
use fuel_tx::crypto::Hasher;
use itertools::Itertools;

use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;

#[derive(Debug, Clone)]
pub struct MemoryStorage {
    block_height: u32,
    coinbase: Address,
    contracts: HashMap<ContractId, Contract>,
    balances: HashMap<(ContractId, Color), Word>,
    contract_state: HashMap<(ContractId, Bytes32), Bytes32>,
    contract_code_root: HashMap<ContractId, (Salt, Bytes32)>,
}

impl MemoryStorage {
    pub fn new(block_height: u32, coinbase: Address) -> Self {
        Self {
            block_height,
            coinbase,
            contracts: Default::default(),
            balances: Default::default(),
            contract_state: Default::default(),
            contract_code_root: Default::default(),
        }
    }

    pub fn contract_state(&self, contract: &ContractId, key: &Bytes32) -> Cow<'_, Bytes32> {
        const DEFAULT_STATE: Bytes32 = Bytes32::zeroed();

        <Self as MerkleStorage<ContractId, Bytes32, Bytes32, Infallible>>::get(&self, contract, key)
            .expect("Infallible")
            .unwrap_or(Cow::Borrowed(&DEFAULT_STATE))
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        let block_height = 1;
        let coinbase = Address::from(*Hasher::hash(b"coinbase"));

        Self::new(block_height, coinbase)
    }
}

impl Storage<ContractId, Contract, Infallible> for MemoryStorage {
    fn insert(&mut self, key: &ContractId, value: &Contract) -> Result<Option<Contract>, Infallible> {
        Ok(self.contracts.insert(*key, value.clone()))
    }

    fn remove(&mut self, key: &ContractId) -> Result<Option<Contract>, Infallible> {
        Ok(self.contracts.remove(key))
    }

    fn get(&self, key: &ContractId) -> Result<Option<Cow<'_, Contract>>, Infallible> {
        Ok(self.contracts.get(key).map(Cow::Borrowed))
    }

    fn contains_key(&self, key: &ContractId) -> Result<bool, Infallible> {
        Ok(self.contracts.contains_key(key))
    }
}

impl Storage<ContractId, (Salt, Bytes32), Infallible> for MemoryStorage {
    fn insert(&mut self, key: &ContractId, value: &(Salt, Bytes32)) -> Result<Option<(Salt, Bytes32)>, Infallible> {
        Ok(self.contract_code_root.insert(*key, *value))
    }

    fn remove(&mut self, key: &ContractId) -> Result<Option<(Salt, Bytes32)>, Infallible> {
        Ok(self.contract_code_root.remove(key))
    }

    fn get(&self, key: &ContractId) -> Result<Option<Cow<'_, (Salt, Bytes32)>>, Infallible> {
        Ok(self.contract_code_root.get(key).map(Cow::Borrowed))
    }

    fn contains_key(&self, key: &ContractId) -> Result<bool, Infallible> {
        Ok(self.contract_code_root.contains_key(key))
    }
}

impl MerkleStorage<ContractId, Color, Word, Infallible> for MemoryStorage {
    fn insert(&mut self, parent: &ContractId, key: &Color, value: &Word) -> Result<Option<Word>, Infallible> {
        Ok(self.balances.insert((*parent, *key), *value))
    }

    fn get(&self, parent: &ContractId, key: &Color) -> Result<Option<Cow<'_, Word>>, Infallible> {
        Ok(self.balances.get(&(*parent, *key)).copied().map(Cow::Owned))
    }

    fn remove(&mut self, parent: &ContractId, key: &Color) -> Result<Option<Word>, Infallible> {
        Ok(self.balances.remove(&(*parent, *key)))
    }

    fn contains_key(&self, parent: &ContractId, key: &Color) -> Result<bool, Infallible> {
        Ok(self.balances.contains_key(&(*parent, *key)))
    }

    fn root(&mut self, parent: &ContractId) -> Result<Bytes32, Infallible> {
        let root = self
            .balances
            .iter()
            .filter_map(|((contract, color), balance)| (contract == parent).then(|| (color, balance)))
            .sorted_by_key(|t| t.0)
            .map(|(_, &balance)| balance)
            .map(Word::to_be_bytes);

        Ok(crypto::ephemeral_merkle_root(root))
    }
}

impl MerkleStorage<ContractId, Bytes32, Bytes32, Infallible> for MemoryStorage {
    fn insert(&mut self, parent: &ContractId, key: &Bytes32, value: &Bytes32) -> Result<Option<Bytes32>, Infallible> {
        Ok(self.contract_state.insert((*parent, *key), *value))
    }

    fn get(&self, parent: &ContractId, key: &Bytes32) -> Result<Option<Cow<'_, Bytes32>>, Infallible> {
        Ok(self.contract_state.get(&(*parent, *key)).map(Cow::Borrowed))
    }

    fn remove(&mut self, parent: &ContractId, key: &Bytes32) -> Result<Option<Bytes32>, Infallible> {
        Ok(self.contract_state.remove(&(*parent, *key)))
    }

    fn contains_key(&self, parent: &ContractId, key: &Bytes32) -> Result<bool, Infallible> {
        Ok(self.contract_state.contains_key(&(*parent, *key)))
    }

    fn root(&mut self, parent: &ContractId) -> Result<Bytes32, Infallible> {
        let root = self
            .contract_state
            .iter()
            .filter_map(|((contract, key), value)| (contract == parent).then(|| (key, value)))
            .sorted_by_key(|t| t.0)
            .map(|(_, value)| value);

        Ok(crypto::ephemeral_merkle_root(root))
    }
}

impl InterpreterStorage for MemoryStorage {
    type Error = Infallible;

    fn block_height(&self) -> Result<u32, Infallible> {
        Ok(self.block_height)
    }

    fn block_hash(&self, block_height: u32) -> Result<Bytes32, Infallible> {
        Ok(Hasher::hash(&block_height.to_be_bytes()))
    }

    fn coinbase(&self) -> Result<Address, Infallible> {
        Ok(self.coinbase)
    }
}

use super::{
    ContractBalanceProvider, ContractCodeProvider, ContractCodeRootProvider, ContractStateProvider, DataError,
    InterpreterStorage, MerkleStorage, Storage,
};
use crate::crypto::{self, Hasher};
use crate::interpreter::Contract;

use fuel_asm::Word;
use fuel_tx::{Address, Bytes32, Color, ContractId, Salt};
use itertools::Itertools;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MemoryStorage {
    block_height: u32,
    coinbase: Address,
    contracts: HashMap<ContractId, Contract>,
    balances: HashMap<(ContractId, Color), Word>,
    contract_state: HashMap<(ContractId, Bytes32), Bytes32>,
    contract_code_root: HashMap<ContractId, (Salt, Bytes32)>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        let block_height = 1;
        let coinbase = Address::from(*Hasher::hash(b"coinbase"));

        Self::new(block_height, coinbase)
    }
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
}

impl Storage<ContractId, Contract> for MemoryStorage {
    fn insert(&mut self, key: ContractId, value: Contract) -> Result<Option<Contract>, DataError> {
        Ok(self.contracts.insert(key, value))
    }

    fn remove(&mut self, key: &ContractId) -> Result<Option<Contract>, DataError> {
        Ok(self.contracts.remove(key))
    }

    fn get(&self, key: &ContractId) -> Result<Option<Contract>, DataError> {
        Ok(self.contracts.get(key).cloned())
    }

    fn contains_key(&self, key: &ContractId) -> Result<bool, DataError> {
        Ok(self.contracts.contains_key(key))
    }
}

impl Storage<ContractId, (Salt, Bytes32)> for MemoryStorage {
    fn insert(&mut self, key: ContractId, value: (Salt, Bytes32)) -> Result<Option<(Salt, Bytes32)>, DataError> {
        Ok(self.contract_code_root.insert(key, value))
    }

    fn remove(&mut self, key: &ContractId) -> Result<Option<(Salt, Bytes32)>, DataError> {
        Ok(self.contract_code_root.remove(key))
    }

    fn get(&self, key: &ContractId) -> Result<Option<(Salt, Bytes32)>, DataError> {
        Ok(self.contract_code_root.get(key).cloned())
    }

    fn contains_key(&self, key: &ContractId) -> Result<bool, DataError> {
        Ok(self.contract_code_root.contains_key(key))
    }
}

impl MerkleStorage<ContractId, Color, Word> for MemoryStorage {
    fn insert(&mut self, parent: &ContractId, key: Color, value: Word) -> Result<Option<Word>, DataError> {
        Ok(self.balances.insert((*parent, key), value))
    }

    fn get(&self, parent: &ContractId, key: &Color) -> Result<Option<Word>, DataError> {
        Ok(self.balances.get(&(*parent, *key)).copied())
    }

    fn remove(&mut self, parent: &ContractId, key: &Color) -> Result<Option<Word>, DataError> {
        Ok(self.balances.remove(&(*parent, *key)))
    }

    fn contains_key(&self, parent: &ContractId, key: &Color) -> Result<bool, DataError> {
        Ok(self.balances.contains_key(&(*parent, *key)))
    }

    fn root(&mut self, parent: &ContractId) -> Result<Bytes32, DataError> {
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

impl MerkleStorage<ContractId, Bytes32, Bytes32> for MemoryStorage {
    fn insert(&mut self, parent: &ContractId, key: Bytes32, value: Bytes32) -> Result<Option<Bytes32>, DataError> {
        Ok(self.contract_state.insert((*parent, key), value))
    }

    fn get(&self, parent: &ContractId, key: &Bytes32) -> Result<Option<Bytes32>, DataError> {
        Ok(self.contract_state.get(&(*parent, *key)).copied())
    }

    fn remove(&mut self, parent: &ContractId, key: &Bytes32) -> Result<Option<Bytes32>, DataError> {
        Ok(self.contract_state.remove(&(*parent, *key)))
    }

    fn contains_key(&self, parent: &ContractId, key: &Bytes32) -> Result<bool, DataError> {
        Ok(self.contract_state.contains_key(&(*parent, *key)))
    }

    fn root(&mut self, parent: &ContractId) -> Result<Bytes32, DataError> {
        let root = self
            .contract_state
            .iter()
            .filter_map(|((contract, key), value)| (contract == parent).then(|| (key, value)))
            .sorted_by_key(|t| t.0)
            .map(|(_, value)| value);

        Ok(crypto::ephemeral_merkle_root(root))
    }
}

impl ContractCodeRootProvider for MemoryStorage {}
impl ContractCodeProvider for MemoryStorage {}
impl ContractBalanceProvider for MemoryStorage {}
impl ContractStateProvider for MemoryStorage {}

impl InterpreterStorage for MemoryStorage {
    fn block_height(&self) -> Result<u32, DataError> {
        Ok(self.block_height)
    }

    fn block_hash(&self, block_height: u32) -> Result<Bytes32, DataError> {
        Ok(Hasher::hash(&block_height.to_be_bytes()))
    }

    fn coinbase(&self) -> Result<Address, DataError> {
        Ok(self.coinbase)
    }
}

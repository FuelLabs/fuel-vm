use super::{DataError, InterpreterStorage, Storage};
use crate::interpreter::Contract;

use fuel_asm::Word;
use fuel_tx::{Bytes32, Color, ContractId};

use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    contracts: HashMap<ContractId, Contract>,
    balances: HashMap<(ContractId, Color), Word>,
    storage: HashMap<(ContractId, Bytes32), Bytes32>,
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

impl Storage<(ContractId, Color), Word> for MemoryStorage {
    fn insert(&mut self, key: (ContractId, Color), value: Word) -> Result<Option<Word>, DataError> {
        Ok(self.balances.insert(key, value))
    }

    fn get(&self, key: &(ContractId, Color)) -> Result<Option<Word>, DataError> {
        Ok(self.balances.get(key).copied())
    }

    fn remove(&mut self, key: &(ContractId, Color)) -> Result<Option<Word>, DataError> {
        Ok(self.balances.remove(key))
    }

    fn contains_key(&self, key: &(ContractId, Color)) -> Result<bool, DataError> {
        Ok(self.balances.contains_key(key))
    }
}

impl Storage<(ContractId, Bytes32), Bytes32> for MemoryStorage {
    fn insert(&mut self, key: (ContractId, Bytes32), value: Bytes32) -> Result<Option<Bytes32>, DataError> {
        Ok(self.storage.insert(key, value))
    }

    fn get(&self, key: &(ContractId, Bytes32)) -> Result<Option<Bytes32>, DataError> {
        Ok(self.storage.get(key).copied())
    }

    fn remove(&mut self, key: &(ContractId, Bytes32)) -> Result<Option<Bytes32>, DataError> {
        Ok(self.storage.remove(key))
    }

    fn contains_key(&self, key: &(ContractId, Bytes32)) -> Result<bool, DataError> {
        Ok(self.storage.contains_key(key))
    }
}

impl InterpreterStorage for MemoryStorage {}

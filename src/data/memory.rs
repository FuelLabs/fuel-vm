use super::{DataError, InterpreterStorage, Storage};
use crate::interpreter::{Contract, ContractColor};

use fuel_asm::Word;
use fuel_tx::ContractId;

use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    contracts: HashMap<ContractId, Contract>,
    balances: HashMap<ContractColor, Word>,
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

impl Storage<ContractColor, Word> for MemoryStorage {
    fn insert(&mut self, key: ContractColor, value: Word) -> Result<Option<Word>, DataError> {
        Ok(self.balances.insert(key, value))
    }

    fn get(&self, key: &ContractColor) -> Result<Option<Word>, DataError> {
        Ok(self.balances.get(key).copied())
    }

    fn remove(&mut self, key: &ContractColor) -> Result<Option<Word>, DataError> {
        Ok(self.balances.remove(key))
    }

    fn contains_key(&self, key: &ContractColor) -> Result<bool, DataError> {
        Ok(self.balances.contains_key(key))
    }
}

impl InterpreterStorage for MemoryStorage {}

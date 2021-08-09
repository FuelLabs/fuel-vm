use super::{DataError, InterpreterStorage, MerkleStorage, Storage};
use crate::interpreter::{BlockData, Contract};

use fuel_asm::Word;
use fuel_tx::{crypto, Address, Bytes32, Color, ContractId};
use itertools::Itertools;

use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    contracts: HashMap<ContractId, Contract>,
    balances: HashMap<(ContractId, Color), Word>,
    storage: HashMap<(ContractId, Bytes32), Bytes32>,
    contract_code_tree: HashMap<Word, [u8; 8]>,
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

impl Storage<Word, [u8; 8]> for MemoryStorage {
    fn insert(&mut self, key: Word, value: [u8; 8]) -> Result<Option<[u8; 8]>, DataError> {
        Ok(self.contract_code_tree.insert(key, value))
    }

    fn get(&self, key: &Word) -> Result<Option<[u8; 8]>, DataError> {
        Ok(self.contract_code_tree.get(key).copied())
    }

    fn remove(&mut self, key: &Word) -> Result<Option<[u8; 8]>, DataError> {
        Ok(self.contract_code_tree.remove(key))
    }

    fn contains_key(&self, key: &Word) -> Result<bool, DataError> {
        Ok(self.contract_code_tree.contains_key(key))
    }
}

impl MerkleStorage<Word, [u8; 8]> for MemoryStorage {
    fn root(&mut self) -> Result<Bytes32, DataError> {
        let bytes = self
            .contract_code_tree
            .drain()
            .sorted_by_key(|entry| entry.0)
            .map(|(_, value)| value)
            .flatten()
            .collect::<Vec<u8>>();

        Ok(crypto::hash(bytes.as_slice()))
    }
}

impl InterpreterStorage for MemoryStorage {
    fn block_height(&self) -> Result<u32, DataError> {
        Ok(1)
    }

    fn coinbase(&self) -> Result<Address, DataError> {
        Ok(Address::from(*crypto::hash(b"coinbase")))
    }

    fn block_data(&self, block_height: u32) -> Result<BlockData, DataError> {
        let hash = crypto::hash(&block_height.to_be_bytes());
        let data = BlockData::new(block_height, hash);

        Ok(data)
    }
}

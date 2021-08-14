use super::{DataError, InterpreterStorage, KeyedMerkleStorage, Storage};
use crate::crypto::{self, Hasher};
use crate::interpreter::{BlockData, Contract, ContractData, ContractState};

use fuel_asm::Word;
use fuel_tx::{Address, Bytes32, Color, ContractId};
use itertools::Itertools;

use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    contracts: HashMap<ContractId, Contract>,
    balances: HashMap<(ContractId, Color), Word>,
    storage: HashMap<(ContractId, Bytes32), Bytes32>,
    contract_data: HashMap<ContractId, ContractData>,
    contract_state: HashMap<ContractId, ContractState>,
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

impl KeyedMerkleStorage<ContractId, (), Color, Word> for MemoryStorage {
    fn initialize(&mut self, _parent: ContractId, _metadata: ()) -> Result<(), DataError> {
        Ok(())
    }

    fn metadata(&self, _parent: &ContractId) -> Result<(), DataError> {
        Ok(())
    }

    fn update(&mut self, _parent: &ContractId, _metadata: ()) -> Result<(), DataError> {
        Ok(())
    }

    fn destroy(&mut self, parent: &ContractId) -> Result<(), DataError> {
        self.balances.retain(|(p, _), _| p != parent);

        Ok(())
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

impl KeyedMerkleStorage<ContractId, (), Bytes32, Bytes32> for MemoryStorage {
    fn initialize(&mut self, _parent: ContractId, _metadata: ()) -> Result<(), DataError> {
        Ok(())
    }

    fn metadata(&self, _parent: &ContractId) -> Result<(), DataError> {
        Ok(())
    }

    fn update(&mut self, _parent: &ContractId, _metadata: ()) -> Result<(), DataError> {
        Ok(())
    }

    fn destroy(&mut self, parent: &ContractId) -> Result<(), DataError> {
        self.storage.retain(|(p, _), _| p != parent);

        Ok(())
    }

    fn root(&mut self, parent: &ContractId) -> Result<Bytes32, DataError> {
        let root = self
            .storage
            .iter()
            .filter_map(|((contract, key), value)| (contract == parent).then(|| (key, value)))
            .sorted_by_key(|t| t.0)
            .map(|(_, value)| value);

        Ok(crypto::ephemeral_merkle_root(root))
    }
}

impl Storage<ContractId, ContractState> for MemoryStorage {
    fn insert(&mut self, key: ContractId, value: ContractState) -> Result<Option<ContractState>, DataError> {
        Ok(self.contract_state.insert(key, value))
    }

    fn get(&self, key: &ContractId) -> Result<Option<ContractState>, DataError> {
        Ok(self.contract_state.get(key).cloned())
    }

    fn remove(&mut self, key: &ContractId) -> Result<Option<ContractState>, DataError> {
        Ok(self.contract_state.remove(key))
    }

    fn contains_key(&self, key: &ContractId) -> Result<bool, DataError> {
        Ok(self.contract_state.contains_key(key))
    }
}

impl Storage<(ContractId, ()), ContractState> for MemoryStorage {
    fn insert(&mut self, key: (ContractId, ()), value: ContractState) -> Result<Option<ContractState>, DataError> {
        Ok(self.contract_state.insert(key.0, value))
    }

    fn get(&self, key: &(ContractId, ())) -> Result<Option<ContractState>, DataError> {
        Ok(self.contract_state.get(&key.0).cloned())
    }

    fn remove(&mut self, key: &(ContractId, ())) -> Result<Option<ContractState>, DataError> {
        Ok(self.contract_state.remove(&key.0))
    }

    fn contains_key(&self, key: &(ContractId, ())) -> Result<bool, DataError> {
        Ok(self.contract_state.contains_key(&key.0))
    }
}

impl KeyedMerkleStorage<ContractId, ContractData, (), ContractState> for MemoryStorage {
    fn initialize(&mut self, _parent: ContractId, _metadata: ContractData) -> Result<(), DataError> {
        Ok(())
    }

    fn metadata(&self, parent: &ContractId) -> Result<ContractData, DataError> {
        self.contract_data
            .get(parent)
            .cloned()
            .ok_or(DataError::MetadataNotAvailable)
    }

    fn update(&mut self, parent: &ContractId, metadata: ContractData) -> Result<(), DataError> {
        self.contracts.insert(*parent, metadata.code().clone());
        self.contract_data.insert(*parent, metadata);

        Ok(())
    }

    fn destroy(&mut self, parent: &ContractId) -> Result<(), DataError> {
        self.contract_data.remove(parent);
        self.contract_state.remove(parent);

        Ok(())
    }

    fn root(&mut self, parent: &ContractId) -> Result<Bytes32, DataError> {
        self.contract_state
            .get(parent)
            .map(|s| ContractData::_root(s.as_ref()))
            .ok_or(DataError::StateNotAvailable)
    }
}

impl InterpreterStorage for MemoryStorage {
    fn block_height(&self) -> Result<u32, DataError> {
        Ok(1)
    }

    fn coinbase(&self) -> Result<Address, DataError> {
        Ok(Address::from(*Hasher::hash(b"coinbase")))
    }

    fn block_data(&self, block_height: u32) -> Result<BlockData, DataError> {
        let hash = Hasher::hash(&block_height.to_be_bytes());
        let data = BlockData::new(block_height, hash);

        Ok(data)
    }
}

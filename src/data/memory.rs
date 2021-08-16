use super::{DataError, InterpreterStorage, KeyedMerkleStorage, Storage};
use crate::crypto::{self, Hasher};
use crate::interpreter::Contract;

use fuel_asm::Word;
use fuel_tx::{Address, Bytes32, Color, ContractId, Salt};
use itertools::Itertools;

use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    contracts: HashMap<ContractId, Contract>,
    balances: HashMap<(ContractId, Color), Word>,
    contract_state: HashMap<(ContractId, Bytes32), Bytes32>,
    contract_code_root: HashMap<ContractId, (Salt, Bytes32)>,
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

impl KeyedMerkleStorage<ContractId, Color, Word> for MemoryStorage {
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
        Ok(self.contract_state.insert(key, value))
    }

    fn get(&self, key: &(ContractId, Bytes32)) -> Result<Option<Bytes32>, DataError> {
        Ok(self.contract_state.get(key).copied())
    }

    fn remove(&mut self, key: &(ContractId, Bytes32)) -> Result<Option<Bytes32>, DataError> {
        Ok(self.contract_state.remove(key))
    }

    fn contains_key(&self, key: &(ContractId, Bytes32)) -> Result<bool, DataError> {
        Ok(self.contract_state.contains_key(key))
    }
}

impl KeyedMerkleStorage<ContractId, Bytes32, Bytes32> for MemoryStorage {
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

impl InterpreterStorage for MemoryStorage {
    type ContractCodeRootProvider = Self;
    type ContractCodeProvider = Self;
    type ContractBalanceProvider = Self;
    type ContractStateProvider = Self;

    fn block_height(&self) -> Result<u32, DataError> {
        Ok(1)
    }

    fn block_hash(&self, block_height: u32) -> Result<Bytes32, DataError> {
        Ok(Hasher::hash(&block_height.to_be_bytes()))
    }

    fn coinbase(&self) -> Result<Address, DataError> {
        Ok(Address::from(*Hasher::hash(b"coinbase")))
    }
}

impl AsRef<Self> for MemoryStorage {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsMut<Self> for MemoryStorage {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

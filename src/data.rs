use crate::contract::Contract;
use crate::error::InterpreterError;

use fuel_data::{Address, Bytes32, Color, ContractId, MerkleStorage, Salt, Storage, Word};

use std::borrow::Cow;
use std::error::Error as StdError;
use std::ops::Deref;

mod memory;

pub use memory::MemoryStorage;

/// When this trait is implemented, the underlying interpreter is guaranteed to
/// have full functionality
pub trait InterpreterStorage:
    Storage<ContractId, Contract, Self::Error>
    + Storage<ContractId, (Salt, Bytes32), Self::Error>
    + MerkleStorage<ContractId, Color, Word, Self::Error>
    + MerkleStorage<ContractId, Bytes32, Bytes32, Self::Error>
{
    type Error: StdError + Into<InterpreterError>;

    fn block_height(&self) -> Result<u32, Self::Error>;
    fn block_hash(&self, block_height: u32) -> Result<Bytes32, Self::Error>;
    fn coinbase(&self) -> Result<Address, Self::Error>;

    fn storage_contract(&self, id: &ContractId) -> Result<Option<Cow<'_, Contract>>, InterpreterError> {
        <Self as Storage<ContractId, Contract, Self::Error>>::get(self, id).map_err(|e| e.into())
    }

    fn storage_contract_insert(
        &mut self,
        id: &ContractId,
        contract: &Contract,
    ) -> Result<Option<Contract>, InterpreterError> {
        <Self as Storage<ContractId, Contract, Self::Error>>::insert(self, id, contract).map_err(|e| e.into())
    }

    fn storage_contract_exists(&self, id: &ContractId) -> Result<bool, InterpreterError> {
        <Self as Storage<ContractId, Contract, Self::Error>>::contains_key(self, id).map_err(|e| e.into())
    }

    fn storage_contract_root(&self, id: &ContractId) -> Result<Option<Cow<'_, (Salt, Bytes32)>>, InterpreterError> {
        <Self as Storage<ContractId, (Salt, Bytes32), Self::Error>>::get(self, id).map_err(|e| e.into())
    }

    fn storage_contract_root_insert(
        &mut self,
        id: &ContractId,
        salt: &Salt,
        root: &Bytes32,
    ) -> Result<Option<(Salt, Bytes32)>, InterpreterError> {
        <Self as Storage<ContractId, (Salt, Bytes32), Self::Error>>::insert(self, id, &(*salt, *root))
            .map_err(|e| e.into())
    }

    fn merkle_contract_state(
        &self,
        id: &ContractId,
        key: &Bytes32,
    ) -> Result<Option<Cow<'_, Bytes32>>, InterpreterError> {
        <Self as MerkleStorage<ContractId, Bytes32, Bytes32, Self::Error>>::get(self, id, key).map_err(|e| e.into())
    }

    fn merkle_contract_state_insert(
        &mut self,
        contract: &ContractId,
        key: &Bytes32,
        value: &Bytes32,
    ) -> Result<Option<Bytes32>, InterpreterError> {
        <Self as MerkleStorage<ContractId, Bytes32, Bytes32, Self::Error>>::insert(self, contract, key, &value)
            .map_err(|e| e.into())
    }

    fn merkle_contract_color_balance(&self, id: &ContractId, color: &Color) -> Result<Option<Word>, InterpreterError> {
        let balance = <Self as MerkleStorage<ContractId, Color, Word, Self::Error>>::get(self, id, color)
            .map_err(|e| e.into())?
            .map(Cow::into_owned);

        Ok(balance)
    }

    fn merkle_contract_color_balance_insert(
        &mut self,
        contract: &ContractId,
        color: &Color,
        value: Word,
    ) -> Result<Option<Word>, InterpreterError> {
        <Self as MerkleStorage<ContractId, Color, Word, Self::Error>>::insert(self, contract, color, &value)
            .map_err(|e| e.into())
    }
}

impl<S> InterpreterStorage for &mut S
where
    S: InterpreterStorage,
{
    type Error = S::Error;

    fn block_height(&self) -> Result<u32, Self::Error> {
        <S as InterpreterStorage>::block_height(self.deref())
    }

    fn block_hash(&self, block_height: u32) -> Result<Bytes32, Self::Error> {
        <S as InterpreterStorage>::block_hash(self.deref(), block_height)
    }

    fn coinbase(&self) -> Result<Address, Self::Error> {
        <S as InterpreterStorage>::coinbase(self.deref())
    }
}

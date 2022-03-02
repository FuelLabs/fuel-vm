//! Trait definitions for storage backend

use crate::contract::Contract;

use fuel_storage::{MerkleStorage, Storage};
use fuel_types::{Address, AssetId, Bytes32, ContractId, Salt, Word};

use std::borrow::Cow;
use std::error::Error as StdError;
use std::io;
use std::ops::Deref;

/// When this trait is implemented, the underlying interpreter is guaranteed to
/// have full functionality
pub trait InterpreterStorage:
    Storage<ContractId, Contract, Error = Self::DataError>
    + Storage<ContractId, (Salt, Bytes32), Error = Self::DataError>
    + MerkleStorage<ContractId, AssetId, Word, Error = Self::DataError>
    + MerkleStorage<ContractId, Bytes32, Bytes32, Error = Self::DataError>
{
    /// Error implementation for reasons unspecified in the protocol.
    type DataError: StdError + Into<io::Error>;

    /// Provide the current block height in which the transactions should be
    /// executed.
    fn block_height(&self) -> Result<u32, Self::DataError>;

    /// Provide the block hash from a given height.
    fn block_hash(&self, block_height: u32) -> Result<Bytes32, Self::DataError>;

    /// Provide the coinbase address for the VM instructions implementation.
    fn coinbase(&self) -> Result<Address, Self::DataError>;

    /// Fetch a previously inserted contract code from the chain state for a
    /// given contract.
    fn storage_contract(&self, id: &ContractId) -> Result<Option<Cow<'_, Contract>>, Self::DataError> {
        <Self as Storage<ContractId, Contract>>::get(self, id)
    }

    /// Append a contract to the chain, provided its identifier.
    ///
    /// Canonically, the identifier should be [`Contract::id`].
    fn storage_contract_insert(
        &mut self,
        id: &ContractId,
        contract: &Contract,
    ) -> Result<Option<Contract>, Self::DataError> {
        <Self as Storage<ContractId, Contract>>::insert(self, id, contract)
    }

    /// Check if a provided contract exists in the chain.
    fn storage_contract_exists(&self, id: &ContractId) -> Result<bool, Self::DataError> {
        <Self as Storage<ContractId, Contract>>::contains_key(self, id)
    }

    /// Fetch a previously inserted salt+root tuple from the chain state for a
    /// given contract.
    fn storage_contract_root(&self, id: &ContractId) -> Result<Option<Cow<'_, (Salt, Bytes32)>>, Self::DataError> {
        <Self as Storage<ContractId, (Salt, Bytes32)>>::get(self, id)
    }

    /// Append the salt+root of a contract that was appended to the chain.
    fn storage_contract_root_insert(
        &mut self,
        id: &ContractId,
        salt: &Salt,
        root: &Bytes32,
    ) -> Result<Option<(Salt, Bytes32)>, Self::DataError> {
        <Self as Storage<ContractId, (Salt, Bytes32)>>::insert(self, id, &(*salt, *root))
    }

    /// Fetch the value form a key-value mapping in a contract storage.
    fn merkle_contract_state(
        &self,
        id: &ContractId,
        key: &Bytes32,
    ) -> Result<Option<Cow<'_, Bytes32>>, Self::DataError> {
        <Self as MerkleStorage<ContractId, Bytes32, Bytes32>>::get(self, id, key)
    }

    /// Insert a key-value mapping in a contract storage.
    fn merkle_contract_state_insert(
        &mut self,
        contract: &ContractId,
        key: &Bytes32,
        value: &Bytes32,
    ) -> Result<Option<Bytes32>, Self::DataError> {
        <Self as MerkleStorage<ContractId, Bytes32, Bytes32>>::insert(self, contract, key, value)
    }

    /// Fetch the balance of an asset ID in a contract storage.
    fn merkle_contract_asset_id_balance(
        &self,
        id: &ContractId,
        asset_id: &AssetId,
    ) -> Result<Option<Word>, Self::DataError> {
        let balance = <Self as MerkleStorage<ContractId, AssetId, Word>>::get(self, id, asset_id)?.map(Cow::into_owned);

        Ok(balance)
    }

    /// Update the balance of an asset ID in a contract storage.
    fn merkle_contract_asset_id_balance_insert(
        &mut self,
        contract: &ContractId,
        asset_id: &AssetId,
        value: Word,
    ) -> Result<Option<Word>, Self::DataError> {
        <Self as MerkleStorage<ContractId, AssetId, Word>>::insert(self, contract, asset_id, &value)
    }
}

impl<S> InterpreterStorage for &mut S
where
    S: InterpreterStorage,
{
    type DataError = S::DataError;

    fn block_height(&self) -> Result<u32, Self::DataError> {
        <S as InterpreterStorage>::block_height(self.deref())
    }

    fn block_hash(&self, block_height: u32) -> Result<Bytes32, Self::DataError> {
        <S as InterpreterStorage>::block_hash(self.deref(), block_height)
    }

    fn coinbase(&self) -> Result<Address, Self::DataError> {
        <S as InterpreterStorage>::coinbase(self.deref())
    }
}

//! Trait definitions for storage backend

use fuel_storage::{MerkleRootStorage, StorageAsMut, StorageAsRef, StorageMutate};
use fuel_tx::Contract;
use fuel_types::{Address, AssetId, Bytes32, ContractId, Salt, Word};

use crate::storage::{ContractsAssets, ContractsInfo, ContractsRawCode, ContractsState};
use std::borrow::Cow;
use std::error::Error as StdError;
use std::io;
use std::ops::Deref;

/// When this trait is implemented, the underlying interpreter is guaranteed to
/// have full functionality
pub trait InterpreterStorage:
    StorageMutate<ContractsRawCode, Error = Self::DataError>
    + StorageMutate<ContractsInfo, Error = Self::DataError>
    + for<'a> MerkleRootStorage<ContractId, ContractsAssets<'a>, Error = Self::DataError>
    + for<'a> MerkleRootStorage<ContractId, ContractsState<'a>, Error = Self::DataError>
    + Sized
{
    /// Error implementation for reasons unspecified in the protocol.
    type DataError: StdError + Into<io::Error>;

    /// Provide the current block height in which the transactions should be
    /// executed.
    fn block_height(&self) -> Result<u32, Self::DataError>;

    /// Return the timestamp of a given block
    ///
    /// This isn't optional because the VM is expected to panic if an invalid block height is
    /// passed - under the assumption that the block height is consistent, the storage should
    /// necessarily have the timestamp for the block, unless some I/O error prevents it from
    /// fetching it.
    fn timestamp(&self, height: u32) -> Result<Word, Self::DataError>;

    /// Provide the block hash from a given height.
    fn block_hash(&self, block_height: u32) -> Result<Bytes32, Self::DataError>;

    /// Provide the coinbase address for the VM instructions implementation.
    fn coinbase(&self) -> Result<Address, Self::DataError>;

    /// Fetch a previously inserted contract code from the chain state for a
    /// given contract.
    fn storage_contract(&self, id: &ContractId) -> Result<Option<Cow<'_, Contract>>, Self::DataError> {
        self.storage::<ContractsRawCode>().get(id)
    }

    /// Append a contract to the chain, provided its identifier.
    ///
    /// Canonically, the identifier should be [`Contract::id`].
    fn storage_contract_insert(
        &mut self,
        id: &ContractId,
        contract: &Contract,
    ) -> Result<Option<Contract>, Self::DataError> {
        self.storage::<ContractsRawCode>().insert(id, contract.as_ref())
    }

    /// Check if a provided contract exists in the chain.
    fn storage_contract_exists(&self, id: &ContractId) -> Result<bool, Self::DataError> {
        self.storage::<ContractsRawCode>().contains_key(id)
    }

    /// Fetch a previously inserted salt+root tuple from the chain state for a
    /// given contract.
    fn storage_contract_root(&self, id: &ContractId) -> Result<Option<Cow<'_, (Salt, Bytes32)>>, Self::DataError> {
        self.storage::<ContractsInfo>().get(id)
    }

    /// Append the salt+root of a contract that was appended to the chain.
    fn storage_contract_root_insert(
        &mut self,
        id: &ContractId,
        salt: &Salt,
        root: &Bytes32,
    ) -> Result<Option<(Salt, Bytes32)>, Self::DataError> {
        self.storage::<ContractsInfo>().insert(id, &(*salt, *root))
    }

    /// Fetch the value form a key-value mapping in a contract storage.
    fn merkle_contract_state(
        &self,
        id: &ContractId,
        key: &Bytes32,
    ) -> Result<Option<Cow<'_, Bytes32>>, Self::DataError> {
        self.storage::<ContractsState>().get(&(id, key))
    }

    /// Insert a key-value mapping in a contract storage.
    fn merkle_contract_state_insert(
        &mut self,
        contract: &ContractId,
        key: &Bytes32,
        value: &Bytes32,
    ) -> Result<Option<Bytes32>, Self::DataError> {
        self.storage::<ContractsState>().insert(&(contract, key), value)
    }

    /// Fetch the balance of an asset ID in a contract storage.
    fn merkle_contract_asset_id_balance(
        &self,
        id: &ContractId,
        asset_id: &AssetId,
    ) -> Result<Option<Word>, Self::DataError> {
        let balance = self
            .storage::<ContractsAssets>()
            .get(&(id, asset_id))?
            .map(Cow::into_owned);

        Ok(balance)
    }

    /// Update the balance of an asset ID in a contract storage.
    fn merkle_contract_asset_id_balance_insert(
        &mut self,
        contract: &ContractId,
        asset_id: &AssetId,
        value: Word,
    ) -> Result<Option<Word>, Self::DataError> {
        self.storage::<ContractsAssets>().insert(&(contract, asset_id), &value)
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

    fn timestamp(&self, height: u32) -> Result<Word, Self::DataError> {
        <S as InterpreterStorage>::timestamp(self.deref(), height)
    }

    fn block_hash(&self, block_height: u32) -> Result<Bytes32, Self::DataError> {
        <S as InterpreterStorage>::block_hash(self.deref(), block_height)
    }

    fn coinbase(&self) -> Result<Address, Self::DataError> {
        <S as InterpreterStorage>::coinbase(self.deref())
    }
}

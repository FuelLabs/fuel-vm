//! Trait definitions for storage backend

use fuel_storage::{MerkleRootStorage, StorageAsRef, StorageInspect, StorageMutate, StorageRead, StorageSize};
use fuel_tx::{Contract, StorageSlot};
use fuel_types::{Address, AssetId, BlockHeight, Bytes32, ContractId, Salt, Word};

use crate::storage::{ContractsAssets, ContractsInfo, ContractsRawCode, ContractsState};
use std::borrow::Cow;
use std::error::Error as StdError;
use std::io;
use std::ops::{Deref, DerefMut};

/// When this trait is implemented, the underlying interpreter is guaranteed to
/// have full functionality
pub trait InterpreterStorage:
    StorageMutate<ContractsRawCode, Error = Self::DataError>
    + StorageSize<ContractsRawCode, Error = Self::DataError>
    + StorageRead<ContractsRawCode, Error = Self::DataError>
    + StorageMutate<ContractsInfo, Error = Self::DataError>
    + MerkleRootStorage<ContractId, ContractsState, Error = Self::DataError>
    + ContractsAssetsStorage<Error = Self::DataError>
{
    /// Error implementation for reasons unspecified in the protocol.
    type DataError: StdError + Into<io::Error>;

    /// Provide the current block height in which the transactions should be
    /// executed.
    fn block_height(&self) -> Result<BlockHeight, Self::DataError>;

    /// Return the timestamp of a given block
    ///
    /// This isn't optional because the VM is expected to panic if an invalid block height is
    /// passed - under the assumption that the block height is consistent, the storage should
    /// necessarily have the timestamp for the block, unless some I/O error prevents it from
    /// fetching it.
    fn timestamp(&self, height: BlockHeight) -> Result<Word, Self::DataError>;

    /// Provide the block hash from a given height.
    fn block_hash(&self, block_height: u32) -> Result<Bytes32, Self::DataError>;

    /// Provide the coinbase address for the VM instructions implementation.
    fn coinbase(&self) -> Result<Address, Self::DataError>;

    /// Deploy a contract into the storage with contract id
    fn deploy_contract_with_id(
        &mut self,
        salt: &Salt,
        slots: &[StorageSlot],
        contract: &Contract,
        root: &Bytes32,
        id: &ContractId,
    ) -> Result<(), Self::DataError> {
        self.storage_contract_insert(id, contract)?;
        self.storage_contract_root_insert(id, salt, root)?;

        slots
            .iter()
            .try_for_each(|s| self.merkle_contract_state_insert(id, s.key(), s.value()).map(|_| ()))
    }

    /// Fetch a previously inserted contract code from the chain state for a
    /// given contract.
    fn storage_contract(&self, id: &ContractId) -> Result<Option<Cow<'_, Contract>>, Self::DataError> {
        StorageInspect::<ContractsRawCode>::get(self, id)
    }

    /// Fetch the size of a previously inserted contract code from the chain state for a
    /// given contract.
    fn storage_contract_size(&self, id: &ContractId) -> Result<Option<usize>, Self::DataError> {
        StorageSize::<ContractsRawCode>::size_of_value(self, id)
    }

    /// Read contract bytes from storage into the buffer.
    fn read_contract(&self, id: &ContractId, writer: &mut [u8]) -> Result<Option<Word>, Self::DataError> {
        Ok(StorageRead::<ContractsRawCode>::read(self, id, writer)?.map(|r| r as Word))
    }

    /// Append a contract to the chain, provided its identifier.
    ///
    /// Canonically, the identifier should be [`Contract::id`].
    fn storage_contract_insert(
        &mut self,
        id: &ContractId,
        contract: &Contract,
    ) -> Result<Option<Contract>, Self::DataError> {
        StorageMutate::<ContractsRawCode>::insert(self, id, contract.as_ref())
    }

    /// Check if a provided contract exists in the chain.
    fn storage_contract_exists(&self, id: &ContractId) -> Result<bool, Self::DataError> {
        self.storage::<ContractsRawCode>().contains_key(id)
    }

    /// Fetch a previously inserted salt+root tuple from the chain state for a
    /// given contract.
    fn storage_contract_root(&self, id: &ContractId) -> Result<Option<Cow<'_, (Salt, Bytes32)>>, Self::DataError> {
        StorageInspect::<ContractsInfo>::get(self, id)
    }

    /// Append the salt+root of a contract that was appended to the chain.
    fn storage_contract_root_insert(
        &mut self,
        id: &ContractId,
        salt: &Salt,
        root: &Bytes32,
    ) -> Result<Option<(Salt, Bytes32)>, Self::DataError> {
        StorageMutate::<ContractsInfo>::insert(self, id, &(*salt, *root))
    }

    /// Fetch the value form a key-value mapping in a contract storage.
    fn merkle_contract_state(
        &self,
        id: &ContractId,
        key: &Bytes32,
    ) -> Result<Option<Cow<'_, Bytes32>>, Self::DataError> {
        StorageInspect::<ContractsState>::get(self, &(id, key).into())
    }

    /// Insert a key-value mapping in a contract storage.
    fn merkle_contract_state_insert(
        &mut self,
        contract: &ContractId,
        key: &Bytes32,
        value: &Bytes32,
    ) -> Result<Option<Bytes32>, Self::DataError> {
        StorageMutate::<ContractsState>::insert(self, &(contract, key).into(), value)
    }

    /// Remove a key-value mapping from a contract storage.
    fn merkle_contract_state_remove(
        &mut self,
        contract: &ContractId,
        key: &Bytes32,
    ) -> Result<Option<Bytes32>, Self::DataError> {
        StorageMutate::<ContractsState>::remove(self, &(contract, key).into())
    }

    /// Fetch a range of values from a key-value mapping in a contract storage.
    /// Returns the full range requested using optional values in case
    /// a requested slot is unset.  
    fn merkle_contract_state_range(
        &self,
        id: &ContractId,
        start_key: &Bytes32,
        range: Word,
    ) -> Result<Vec<Option<Cow<Bytes32>>>, Self::DataError>;

    /// Insert a range of key-value mappings into contract storage.
    /// Returns None if any of the keys in the range were previously unset.
    fn merkle_contract_state_insert_range(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        values: &[Bytes32],
    ) -> Result<Option<()>, Self::DataError>;

    /// Remove a range of key-values from contract storage.
    /// Returns None if any of the keys in the range were already unset.
    fn merkle_contract_state_remove_range(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        range: Word,
    ) -> Result<Option<()>, Self::DataError>;
}

/// Storage operations for contract assets.
pub trait ContractsAssetsStorage: MerkleRootStorage<ContractId, ContractsAssets> {
    /// Fetch the balance of an asset ID in a contract storage.
    fn merkle_contract_asset_id_balance(
        &self,
        id: &ContractId,
        asset_id: &AssetId,
    ) -> Result<Option<Word>, Self::Error> {
        let balance = self
            .storage::<ContractsAssets>()
            .get(&(id, asset_id).into())?
            .map(Cow::into_owned);

        Ok(balance)
    }

    /// Update the balance of an asset ID in a contract storage.
    fn merkle_contract_asset_id_balance_insert(
        &mut self,
        contract: &ContractId,
        asset_id: &AssetId,
        value: Word,
    ) -> Result<Option<Word>, Self::Error> {
        StorageMutate::<ContractsAssets>::insert(self, &(contract, asset_id).into(), &value)
    }
}

impl<S> ContractsAssetsStorage for &mut S where S: ContractsAssetsStorage {}

impl<S> InterpreterStorage for &mut S
where
    S: InterpreterStorage,
{
    type DataError = S::DataError;

    fn block_height(&self) -> Result<BlockHeight, Self::DataError> {
        <S as InterpreterStorage>::block_height(self.deref())
    }

    fn timestamp(&self, height: BlockHeight) -> Result<Word, Self::DataError> {
        <S as InterpreterStorage>::timestamp(self.deref(), height)
    }

    fn block_hash(&self, block_height: u32) -> Result<Bytes32, Self::DataError> {
        <S as InterpreterStorage>::block_hash(self.deref(), block_height)
    }

    fn coinbase(&self) -> Result<Address, Self::DataError> {
        <S as InterpreterStorage>::coinbase(self.deref())
    }

    fn storage_contract_size(&self, id: &ContractId) -> Result<Option<usize>, Self::DataError> {
        <S as InterpreterStorage>::storage_contract_size(self.deref(), id)
    }

    fn read_contract(&self, id: &ContractId, writer: &mut [u8]) -> Result<Option<Word>, Self::DataError> {
        <S as InterpreterStorage>::read_contract(self.deref(), id, writer)
    }

    fn merkle_contract_state_range(
        &self,
        id: &ContractId,
        start_key: &Bytes32,
        range: Word,
    ) -> Result<Vec<Option<Cow<Bytes32>>>, Self::DataError> {
        <S as InterpreterStorage>::merkle_contract_state_range(self.deref(), id, start_key, range)
    }

    fn merkle_contract_state_insert_range(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        values: &[Bytes32],
    ) -> Result<Option<()>, Self::DataError> {
        <S as InterpreterStorage>::merkle_contract_state_insert_range(self.deref_mut(), contract, start_key, values)
    }

    fn merkle_contract_state_remove_range(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        range: Word,
    ) -> Result<Option<()>, Self::DataError> {
        <S as InterpreterStorage>::merkle_contract_state_remove_range(self.deref_mut(), contract, start_key, range)
    }
}

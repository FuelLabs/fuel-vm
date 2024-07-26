//! Trait definitions for storage backend

use fuel_storage::{
    StorageAsRef,
    StorageInspect,
    StorageMutate,
    StorageRead,
    StorageSize,
    StorageWrite,
};
use fuel_tx::{
    ConsensusParameters,
    Contract,
    StorageSlot,
};
use fuel_types::{
    AssetId,
    BlockHeight,
    Bytes32,
    ContractId,
    Word,
};

use crate::{
    prelude::{
        InterpreterError,
        RuntimeError,
    },
    storage::{
        ContractsAssets,
        ContractsRawCode,
        ContractsState,
        ContractsStateData,
        UploadedBytecode,
        UploadedBytecodes,
    },
};
use alloc::{
    borrow::Cow,
    vec::Vec,
};
use core::ops::{
    Deref,
    DerefMut,
};

use super::blob_data::BlobData;

/// When this trait is implemented, the underlying interpreter is guaranteed to
/// have full functionality
pub trait InterpreterStorage:
    StorageWrite<ContractsRawCode, Error = Self::DataError>
    + StorageSize<ContractsRawCode, Error = Self::DataError>
    + StorageRead<ContractsRawCode, Error = Self::DataError>
    + StorageWrite<ContractsState, Error = Self::DataError>
    + StorageSize<ContractsState, Error = Self::DataError>
    + StorageRead<ContractsState, Error = Self::DataError>
    + StorageMutate<UploadedBytecodes, Error = Self::DataError>
    + StorageWrite<BlobData, Error = Self::DataError>
    + StorageSize<BlobData, Error = Self::DataError>
    + StorageRead<BlobData, Error = Self::DataError>
    + ContractsAssetsStorage<Error = Self::DataError>
{
    /// Error implementation for reasons unspecified in the protocol.
    type DataError: Into<InterpreterError<Self::DataError>>
        + Into<RuntimeError<Self::DataError>>
        + core::fmt::Debug;

    /// Provide the current block height in which the transactions should be
    /// executed.
    fn block_height(&self) -> Result<BlockHeight, Self::DataError>;

    /// Provide the current version of consensus parameters used to execute transaction.
    fn consensus_parameters_version(&self) -> Result<u32, Self::DataError>;

    /// Provide the current version of state transition function used to execute
    /// transaction.
    fn state_transition_version(&self) -> Result<u32, Self::DataError>;

    /// Return the timestamp of a given block
    ///
    /// This isn't optional because the VM is expected to panic if an invalid block height
    /// is passed - under the assumption that the block height is consistent, the
    /// storage should necessarily have the timestamp for the block, unless some I/O
    /// error prevents it from fetching it.
    fn timestamp(&self, height: BlockHeight) -> Result<Word, Self::DataError>;

    /// Provide the block hash from a given height.
    fn block_hash(&self, block_height: BlockHeight) -> Result<Bytes32, Self::DataError>;

    /// Provide the coinbase address for the VM instructions implementation.
    fn coinbase(&self) -> Result<ContractId, Self::DataError>;

    /// Set the consensus parameters in the storage under the `version`.
    ///
    /// Returns the previous consensus parameters if they were set.
    fn set_consensus_parameters(
        &mut self,
        version: u32,
        consensus_parameters: &ConsensusParameters,
    ) -> Result<Option<ConsensusParameters>, Self::DataError>;

    /// Returns `true` if the fully uploaded state transition bytecode is present in the
    /// storage.
    fn contains_state_transition_bytecode_root(
        &self,
        root: &Bytes32,
    ) -> Result<bool, Self::DataError> {
        let bytecode = self.storage::<UploadedBytecodes>().get(root)?;

        if let Some(cow) = bytecode {
            if let UploadedBytecode::Completed(_) = cow.as_ref() {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Set the state transition bytecode in the storage under the `version`.
    ///
    /// Returns the previous bytecode if it was set.
    fn set_state_transition_bytecode(
        &mut self,
        version: u32,
        hash: &Bytes32,
    ) -> Result<Option<Bytes32>, Self::DataError>;

    /// Deploy a contract into the storage with contract id
    fn deploy_contract_with_id(
        &mut self,
        slots: &[StorageSlot],
        contract: &Contract,
        id: &ContractId,
    ) -> Result<(), Self::DataError> {
        self.storage_contract_insert(id, contract)?;

        // On the `fuel-core` side it is done in more optimal way
        slots.iter().try_for_each(|s| {
            self.contract_state_insert(id, s.key(), s.value().as_ref())?;
            Ok(())
        })?;
        Ok(())
    }

    /// Fetch a previously inserted contract code from the chain state for a
    /// given contract.
    fn storage_contract(
        &self,
        id: &ContractId,
    ) -> Result<Option<Cow<'_, Contract>>, Self::DataError> {
        StorageInspect::<ContractsRawCode>::get(self, id)
    }

    /// Fetch the size of a previously inserted contract code from the chain state for a
    /// given contract.
    fn storage_contract_size(
        &self,
        id: &ContractId,
    ) -> Result<Option<usize>, Self::DataError> {
        StorageSize::<ContractsRawCode>::size_of_value(self, id)
    }

    /// Read contract bytes from storage into the buffer.
    fn read_contract(
        &self,
        id: &ContractId,
        writer: &mut [u8],
    ) -> Result<Option<Word>, Self::DataError> {
        Ok(StorageRead::<ContractsRawCode>::read(self, id, writer)?.map(|r| r as Word))
    }

    /// Append a contract to the chain, provided its identifier.
    ///
    /// Canonically, the identifier should be [`Contract::id`].
    fn storage_contract_insert(
        &mut self,
        id: &ContractId,
        contract: &Contract,
    ) -> Result<(), Self::DataError> {
        StorageMutate::<ContractsRawCode>::insert(self, id, contract.as_ref())
    }

    /// Check if a provided contract exists in the chain.
    fn storage_contract_exists(&self, id: &ContractId) -> Result<bool, Self::DataError> {
        self.storage::<ContractsRawCode>().contains_key(id)
    }

    /// Fetch the value form a key-value mapping in a contract storage.
    fn contract_state(
        &self,
        id: &ContractId,
        key: &Bytes32,
    ) -> Result<Option<Cow<'_, ContractsStateData>>, Self::DataError> {
        StorageInspect::<ContractsState>::get(self, &(id, key).into())
    }

    /// Insert a key-value mapping in a contract storage.
    fn contract_state_insert(
        &mut self,
        contract: &ContractId,
        key: &Bytes32,
        value: &[u8],
    ) -> Result<(), Self::DataError> {
        StorageWrite::<ContractsState>::write_bytes(
            self,
            &(contract, key).into(),
            value,
        )?;
        Ok(())
    }

    /// Insert a key-value mapping into a contract storage.
    fn contract_state_replace(
        &mut self,
        contract: &ContractId,
        key: &Bytes32,
        value: &[u8],
    ) -> Result<Option<Vec<u8>>, Self::DataError> {
        let (_, prev) = StorageWrite::<ContractsState>::replace_bytes(
            self,
            &(contract, key).into(),
            value,
        )?;
        Ok(prev)
    }

    /// Fetch a range of values from a key-value mapping in a contract storage.
    /// Returns the full range requested using optional values in case
    /// a requested slot is unset.  
    fn contract_state_range(
        &self,
        id: &ContractId,
        start_key: &Bytes32,
        range: usize,
    ) -> Result<Vec<Option<Cow<ContractsStateData>>>, Self::DataError>;

    /// Insert a range of key-value mappings into contract storage.
    /// Returns the number of keys that were previously unset but are now set.
    fn contract_state_insert_range<'a, I>(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        values: I,
    ) -> Result<usize, Self::DataError>
    where
        I: Iterator<Item = &'a [u8]>;

    /// Remove a range of key-values from contract storage.
    /// Returns None if any of the keys in the range were already unset.
    fn contract_state_remove_range(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        range: usize,
    ) -> Result<Option<()>, Self::DataError>;
}

/// Storage operations for contract assets.
pub trait ContractsAssetsStorage: StorageMutate<ContractsAssets> {
    /// Fetch the balance of an asset ID in a contract storage.
    fn contract_asset_id_balance(
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
    fn contract_asset_id_balance_insert(
        &mut self,
        contract: &ContractId,
        asset_id: &AssetId,
        value: Word,
    ) -> Result<(), Self::Error> {
        StorageMutate::<ContractsAssets>::insert(
            self,
            &(contract, asset_id).into(),
            &value,
        )
    }

    /// Update the balance of an asset ID in a contract storage.
    /// Returns the old balance, if any.
    fn contract_asset_id_balance_replace(
        &mut self,
        contract: &ContractId,
        asset_id: &AssetId,
        value: Word,
    ) -> Result<Option<Word>, Self::Error> {
        StorageMutate::<ContractsAssets>::replace(
            self,
            &(contract, asset_id).into(),
            &value,
        )
    }
}

impl<S> ContractsAssetsStorage for &mut S where S: ContractsAssetsStorage {}

impl<S> InterpreterStorage for &mut S
where
    S: InterpreterStorage,
{
    type DataError = <S as InterpreterStorage>::DataError;

    fn block_height(&self) -> Result<BlockHeight, Self::DataError> {
        <S as InterpreterStorage>::block_height(self.deref())
    }

    fn consensus_parameters_version(&self) -> Result<u32, Self::DataError> {
        <S as InterpreterStorage>::consensus_parameters_version(self.deref())
    }

    fn state_transition_version(&self) -> Result<u32, Self::DataError> {
        <S as InterpreterStorage>::state_transition_version(self.deref())
    }

    fn timestamp(&self, height: BlockHeight) -> Result<Word, Self::DataError> {
        <S as InterpreterStorage>::timestamp(self.deref(), height)
    }

    fn block_hash(&self, block_height: BlockHeight) -> Result<Bytes32, Self::DataError> {
        <S as InterpreterStorage>::block_hash(self.deref(), block_height)
    }

    fn coinbase(&self) -> Result<ContractId, Self::DataError> {
        <S as InterpreterStorage>::coinbase(self.deref())
    }

    fn set_consensus_parameters(
        &mut self,
        version: u32,
        consensus_parameters: &ConsensusParameters,
    ) -> Result<Option<ConsensusParameters>, Self::DataError> {
        <S as InterpreterStorage>::set_consensus_parameters(
            self.deref_mut(),
            version,
            consensus_parameters,
        )
    }

    fn set_state_transition_bytecode(
        &mut self,
        version: u32,
        hash: &Bytes32,
    ) -> Result<Option<Bytes32>, Self::DataError> {
        <S as InterpreterStorage>::set_state_transition_bytecode(
            self.deref_mut(),
            version,
            hash,
        )
    }

    fn storage_contract_size(
        &self,
        id: &ContractId,
    ) -> Result<Option<usize>, Self::DataError> {
        <S as InterpreterStorage>::storage_contract_size(self.deref(), id)
    }

    fn read_contract(
        &self,
        id: &ContractId,
        writer: &mut [u8],
    ) -> Result<Option<Word>, Self::DataError> {
        <S as InterpreterStorage>::read_contract(self.deref(), id, writer)
    }

    fn contract_state_range(
        &self,
        id: &ContractId,
        start_key: &Bytes32,
        range: usize,
    ) -> Result<Vec<Option<Cow<ContractsStateData>>>, Self::DataError> {
        <S as InterpreterStorage>::contract_state_range(
            self.deref(),
            id,
            start_key,
            range,
        )
    }

    fn contract_state_insert_range<'a, I>(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        values: I,
    ) -> Result<usize, Self::DataError>
    where
        I: Iterator<Item = &'a [u8]>,
    {
        <S as InterpreterStorage>::contract_state_insert_range(
            self.deref_mut(),
            contract,
            start_key,
            values,
        )
    }

    fn contract_state_remove_range(
        &mut self,
        contract: &ContractId,
        start_key: &Bytes32,
        range: usize,
    ) -> Result<Option<()>, Self::DataError> {
        <S as InterpreterStorage>::contract_state_remove_range(
            self.deref_mut(),
            contract,
            start_key,
            range,
        )
    }
}

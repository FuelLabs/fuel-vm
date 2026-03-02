use fuel_asm::RegId;
use fuel_storage::{
    StorageRead,
    StorageReadError,
};
use fuel_tx::{
    Bytes32,
    ContractId,
    PanicReason,
};

use crate::{
    convert,
    error::{
        IoResult,
        RuntimeError,
    },
    prelude::{
        Interpreter,
        Memory,
    },
    storage::{
        ContractsState,
        ContractsStateKey,
        InterpreterStorage,
    },
};

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
{
    /// When size of a slot is known, cache it.
    /// See documentation on [`Interpreter::storage_slot_size_cache`] field.
    pub(crate) fn cache_size_of_slot(
        &mut self,
        contract_id: ContractId,
        key: Bytes32,
        size: u64,
    ) {
        let cache_key = (contract_id, key);
        self.storage_slot_size_cache.insert(cache_key, size);
    }

    /// Gets the size of the storage slot identified by `key`, using the cache if
    /// available. See documentation on [`Interpreter::storage_slot_size_cache`]
    /// field.
    pub(crate) fn get_size_of_slot_cached(
        &mut self,
        key: Bytes32,
    ) -> Result<u64, RuntimeError<S::DataError>> {
        let contract_id = self.internal_contract()?;
        let cache_key = (contract_id, key);

        if let Some(size) = self.storage_slot_size_cache.get(&cache_key) {
            return Ok(*size);
        }

        let size = self
            .storage
            .contract_state(&contract_id, &key)
            .map_err(RuntimeError::Storage)?
            .map(|v| v.as_ref().0.len() as u64)
            .unwrap_or(0);

        self.cache_size_of_slot(contract_id, key, size);
        Ok(size)
    }

    /// Verifies that the given size does not exceed the maximum allowed storage slot
    /// length.
    pub(crate) fn verify_storage_smaller_than_max(
        &self,
        size: usize,
    ) -> Result<(), RuntimeError<S::DataError>> {
        let max_size = self.interpreter_params.max_storage_slot_length;
        if (size as u64) > max_size {
            return Err(RuntimeError::Recoverable(PanicReason::StorageOutOfBounds));
        }
        Ok(())
    }

    /// Returns length of the value, or 0 if the slot is not found.
    pub(crate) fn storage_read_to_memory(
        &mut self,
        key: Bytes32,
        dst_ptr: u64,
        offset: u64,
        len: u64,
    ) -> Result<u64, RuntimeError<S::DataError>> {
        let offset = convert::to_usize(offset).ok_or(PanicReason::MemoryOverflow)?;

        let len = convert::to_usize(len).ok_or(PanicReason::MemoryOverflow)?;

        let contract_id = self.internal_contract()?;
        let owner = self.ownership_registers();

        let dst = self.memory.as_mut().write(owner, dst_ptr, len)?;

        match StorageRead::<ContractsState>::read_exact(
            &self.storage,
            &ContractsStateKey::new(&contract_id, &key),
            offset,
            dst,
        )
        .map_err(RuntimeError::Storage)?
        {
            Ok(total_len) => {
                self.cache_size_of_slot(contract_id, key, total_len as u64);
                self.registers[RegId::ERR] = 0;
                Ok(total_len as u64)
            }
            Err(StorageReadError::KeyNotFound) => {
                self.registers[RegId::ERR] = 1;
                Ok(0)
            }
            Err(StorageReadError::OutOfBounds) => {
                Err(RuntimeError::Recoverable(PanicReason::StorageOutOfBounds))
            }
        }
    }

    pub(crate) fn storage_write_from_memory(
        &mut self,
        key: Bytes32,
        src_ptr: u64,
        len: u64,
    ) -> Result<(), RuntimeError<S::DataError>> {
        let contract_id = self.internal_contract()?;
        let len = convert::to_usize(len).ok_or(PanicReason::MemoryOverflow)?;
        self.verify_storage_smaller_than_max(len)?;
        self.cache_size_of_slot(contract_id, key, len as u64);
        let src = self.memory.as_mut().read(src_ptr, len)?;
        self.storage
            .contract_state_insert(&contract_id, &key, src)
            .map_err(RuntimeError::Storage)
    }

    /// Storage read, subslice update and write back.
    /// Returns max of read length and resulting slot length.
    pub(crate) fn storage_update_from_memory(
        &mut self,
        key: Bytes32,
        src_ptr: u64,
        offset: u64,
        write_len: u64,
    ) -> Result<StorageUpdated, RuntimeError<S::DataError>> {
        let contract_id = self.internal_contract()?;
        let mut value = self
            .storage
            .contract_state(&contract_id, &key)
            .map_err(RuntimeError::Storage)?
            .map(|v| v.as_ref().0.clone().into_inner())
            .unwrap_or_default();

        let total_size_before = value.len() as u64;

        let offset = if offset == u64::MAX {
            value.len()
        } else {
            convert::to_usize(offset).ok_or(PanicReason::MemoryOverflow)?
        };

        if offset > value.len() {
            return Err(RuntimeError::Recoverable(PanicReason::StorageOutOfBounds));
        }

        let write_len =
            convert::to_usize(write_len).ok_or(PanicReason::MemoryOverflow)?;
        let len_after = offset.saturating_add(write_len);

        self.verify_storage_smaller_than_max(len_after)?;

        if len_after > value.len() {
            value.resize(len_after, 0);
        }

        value[offset..len_after]
            .copy_from_slice(self.memory.as_mut().read(src_ptr, write_len)?);

        self.storage
            .contract_state_insert(&contract_id, &key, &value)
            .map_err(RuntimeError::Storage)?;

        let total_size_after = len_after as u64;
        self.cache_size_of_slot(contract_id, key, total_size_after);

        Ok(StorageUpdated {
            total_size_before,
            total_size_after,
        })
    }

    /// Implementation of SRDD/SRDI opcodes
    pub(crate) fn dynamic_storage_read(
        &mut self,
        buffer_ptr: u64,
        key_ptr: u64,
        offset: u64,
        len: u64,
    ) -> IoResult<(), S::DataError> {
        let key = Bytes32::from(self.memory().read_bytes(key_ptr)?);
        let len = self.storage_read_to_memory(key, buffer_ptr, offset, len)?;
        self.dependent_gas_charge(
            self.gas_costs().srdd().map_err(PanicReason::from)?,
            len,
        )?;
        Ok(())
    }

    /// Implementation of SWRD/SWRI opcodes
    pub(crate) fn dynamic_storage_write(
        &mut self,
        key_ptr: u64,
        value_ptr: u64,
        len: u64,
    ) -> IoResult<(), S::DataError> {
        let key = Bytes32::from(self.memory().read_bytes(key_ptr)?);
        self.dependent_gas_charge(
            self.gas_costs().swrd().map_err(PanicReason::from)?,
            len,
        )?;
        let size_before = self.get_size_of_slot_cached(key)?;
        let new_bytes = len.saturating_sub(size_before);
        if new_bytes > 0 {
            self.gas_charge(
                self.gas_costs()
                    .new_storage_per_byte()
                    .saturating_mul(new_bytes),
            )?;
        }
        self.storage_write_from_memory(key, value_ptr, len)?;
        Ok(())
    }

    /// Implementation of SUPD/SUPI opcodes
    pub(crate) fn dynamic_storage_update(
        &mut self,
        key_ptr: u64,
        value_ptr: u64,
        offset: u64,
        len: u64,
    ) -> IoResult<(), S::DataError> {
        let key = Bytes32::from(self.memory().read_bytes(key_ptr)?);
        let update = self.storage_update_from_memory(key, value_ptr, offset, len)?;
        self.dependent_gas_charge(
            self.gas_costs().supd().map_err(PanicReason::from)?,
            update.transfer_charge(),
        )?;
        if update.new_bytes() > 0 {
            self.gas_charge(
                self.gas_costs()
                    .new_storage_per_byte()
                    .saturating_mul(update.new_bytes()),
            )?;
        }
        Ok(())
    }

    /// Implementation of SPLD opcode
    pub(crate) fn storage_preload(
        &mut self,
        r_dst_len: RegId,
        key: Bytes32,
    ) -> Result<u64, RuntimeError<S::DataError>> {
        let contract_id = self.internal_contract()?;
        let value = self
            .storage
            .contract_state(&contract_id, &key)
            .map_err(RuntimeError::Storage)?;

        let Some(value) = value else {
            self.registers[RegId::ERR] = 1;
            return Ok(0);
        };

        let dst = self.memory.as_mut().storage_preload_mut();
        *dst = value.as_ref().as_ref().to_vec();
        let len = dst.len() as u64;
        self.registers[RegId::ERR] = 0;
        self.cache_size_of_slot(contract_id, key, len);

        self.write_user_register(r_dst_len, len)?;
        self.dependent_gas_charge(
            self.gas_costs().spld().map_err(PanicReason::from)?,
            len,
        )?;
        Ok(len)
    }
}

pub(crate) struct StorageUpdated {
    total_size_before: u64,
    total_size_after: u64,
}
impl StorageUpdated {
    /// The amount to for dynamic gas charge.
    /// We use max here to be conservative.
    pub fn transfer_charge(&self) -> u64 {
        self.total_size_after.max(self.total_size_before)
    }

    /// The amount of new bytes added to storage, used for new storage gas charge.
    pub fn new_bytes(&self) -> u64 {
        self.total_size_after.saturating_sub(self.total_size_before)
    }
}

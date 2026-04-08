use fuel_asm::RegId;
use fuel_storage::StorageRead;
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
        MemoryInstance,
    },
    storage::{
        ContractsState,
        ContractsStateKey,
        InterpreterStorage,
    },
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
{
    pub(crate) fn storage_read_slot<F, R>(
        &mut self,
        contract_id: ContractId,
        key: Bytes32,
        f: F,
    ) -> Result<R, RuntimeError<S::DataError>>
    where
        F: FnOnce(&mut MemoryInstance, Option<&[u8]>) -> R,
    {
        let cache_key = (contract_id, key);

        if let Some(v) = self.storage_slot_cache.get(&cache_key) {
            // Cache hit
            let gas_charge_units = v.as_ref().map(|data| data.len() as u64).unwrap_or(0);
            let r = f(self.memory.as_mut(), v.as_deref());
            self.dependent_gas_charge(
                self.gas_costs()
                    .storage_read_hot()
                    .map_err(PanicReason::from)?,
                gas_charge_units,
            )?;
            return Ok(r);
        }
        let value = StorageRead::<ContractsState>::read_alloc(
            &self.storage,
            &ContractsStateKey::new(&contract_id, &key),
        )
        .map_err(RuntimeError::Storage)?;
        // Cache miss
        let gas_charge_units = value.as_ref().map(|data| data.len() as u64).unwrap_or(0);
        let r = f(self.memory.as_mut(), value.as_deref());
        self.dependent_gas_charge(
            self.gas_costs()
                .storage_read_cold()
                .map_err(PanicReason::from)?,
            gas_charge_units,
        )?;
        self.storage_slot_cache.insert(cache_key, value);
        Ok(r)
    }

    /// Returns the byte length of the current value for a storage slot **without
    /// charging gas**.
    ///
    /// Checks the in-memory cache first (a single `BTreeMap` lookup with no I/O).
    /// On a cold miss it fetches from backing storage and populates the cache so
    /// that any subsequent charged read is hot.
    ///
    /// This is used by [`Self::storage_write_slot`] to determine `old_len` for
    /// the `new_storage_per_byte` charge.  Callers such as `SWW` and `SWWQ`
    /// already perform a charged `storage_read_slot` before calling
    /// `storage_write_slot`, which warms the cache.  Using this helper instead
    /// of a second `storage_read_slot` avoids the extra `storage_read_hot`
    /// charge that would otherwise be incurred on every write.
    fn storage_slot_len_no_gas(
        &mut self,
        contract_id: ContractId,
        key: Bytes32,
    ) -> Result<usize, RuntimeError<S::DataError>> {
        let cache_key = (contract_id, key);
        if let Some(v) = self.storage_slot_cache.get(&cache_key) {
            return Ok(v.as_ref().map(|d| d.len()).unwrap_or(0));
        }
        let value = StorageRead::<ContractsState>::read_alloc(
            &self.storage,
            &ContractsStateKey::new(&contract_id, &key),
        )
        .map_err(RuntimeError::Storage)?;
        let len = value.as_ref().map(|d| d.len()).unwrap_or(0);
        self.storage_slot_cache.insert(cache_key, value);
        Ok(len)
    }

    pub(crate) fn storage_write_slot(
        &mut self,
        contract_id: ContractId,
        key: Bytes32,
        value: Vec<u8>,
    ) -> Result<(), RuntimeError<S::DataError>> {
        let old_len = self.storage_slot_len_no_gas(contract_id, key)?;
        let max_size = self.interpreter_params.max_storage_slot_length;
        if (value.len() as u64) > max_size {
            return Err(RuntimeError::Recoverable(PanicReason::StorageOutOfBounds));
        }
        let cache_key = (contract_id, key);
        self.storage
            .contract_state_insert(&contract_id, &key, &value)
            .map_err(RuntimeError::Storage)?;
        let gas_charge_units = value.len() as u64;
        self.storage_slot_cache.insert(cache_key, Some(value));
        self.dependent_gas_charge(
            self.gas_costs()
                .storage_write()
                .map_err(PanicReason::from)?,
            gas_charge_units,
        )?;
        self.gas_charge(
            self.gas_costs()
                .new_storage_per_byte()
                .saturating_mul(gas_charge_units.saturating_sub(old_len as u64)),
        )?;
        Ok(())
    }

    pub(crate) fn storage_write_slot_from_memory<F>(
        &mut self,
        contract_id: ContractId,
        key: Bytes32,
        f: F,
    ) -> Result<(), RuntimeError<S::DataError>>
    where
        F: FnOnce(&MemoryInstance) -> Result<&[u8], RuntimeError<S::DataError>>,
    {
        // Copy to an owned buffer so the immutable borrow of `self.memory`
        // ends before the mutable borrow required by `storage_write_slot`.
        let value = f(self.memory.as_ref())?.to_vec();
        self.storage_write_slot(contract_id, key, value)
    }

    pub(crate) fn storage_clear_slot_range(
        &mut self,
        contract_id: ContractId,
        key: Bytes32,
        range: usize,
    ) -> Result<(), RuntimeError<S::DataError>> {
        // Ensure the key range doesn't overflow U256. A range of 0 or 1 starting at
        // any key is always valid; for larger ranges we check that
        // start_key + (range - 1) doesn't wrap around.
        if range > 1 {
            let start = primitive_types::U256::from_big_endian(&*key);
            start
                .checked_add(primitive_types::U256::from(range - 1))
                .ok_or(PanicReason::TooManySlots)?;
        }
        self.dependent_gas_charge(
            self.gas_costs()
                .storage_clear()
                .map_err(PanicReason::from)?,
            range as u64,
        )?;
        self.storage
            .contract_state_remove_range(&contract_id, &key, range)
            .map_err(RuntimeError::Storage)?;
        for key in key_range(key, range) {
            let key = key.ok_or(PanicReason::TooManySlots)?;
            let cache_key = (contract_id, key);
            self.storage_slot_cache.insert(cache_key, None);
        }
        Ok(())
    }

    pub(crate) fn storage_read_to_memory(
        &mut self,
        contract_id: ContractId,
        key: Bytes32,
        dst_ptr: u64,
        offset: u64,
        len: u64,
    ) -> Result<(), RuntimeError<S::DataError>> {
        let offset = convert::to_usize(offset).ok_or(PanicReason::MemoryOverflow)?;

        let len = convert::to_usize(len).ok_or(PanicReason::MemoryOverflow)?;

        let owner = self.ownership_registers();
        self.registers[RegId::ERR] = self
            .storage_read_slot::<_, Result<u64, RuntimeError<S::DataError>>>(
                contract_id,
                key,
                |memory, value| match value {
                    Some(value) => {
                        let src = value.get(offset..offset.saturating_add(len)).ok_or(
                            RuntimeError::Recoverable(PanicReason::StorageOutOfBounds),
                        )?;

                        let dst = memory.write(owner, dst_ptr, len)?;
                        dst.copy_from_slice(src);
                        Ok(0)
                    }
                    None => Ok(1),
                },
            )??;
        Ok(())
    }

    pub(crate) fn storage_write_from_memory(
        &mut self,
        contract_id: ContractId,
        key: Bytes32,
        src_ptr: u64,
        len: u64,
    ) -> Result<(), RuntimeError<S::DataError>> {
        let len = convert::to_usize(len).ok_or(PanicReason::MemoryOverflow)?;
        self.storage_write_slot_from_memory(contract_id, key, |memory| {
            Ok(memory.read(src_ptr, len)?)
        })
    }

    /// Storage read, subslice update and write back.
    /// Returns max of read length and resulting slot length.
    pub(crate) fn storage_update_from_memory(
        &mut self,
        contract_id: ContractId,
        key: Bytes32,
        src_ptr: u64,
        offset: u64,
        write_len: u64,
    ) -> Result<(), RuntimeError<S::DataError>> {
        let mut value = self
            .storage_read_slot(contract_id, key, |_, v| v.unwrap_or_default().to_vec())?;

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

        let max_size = self.interpreter_params.max_storage_slot_length;
        if (len_after as u64) > max_size {
            return Err(RuntimeError::Recoverable(PanicReason::StorageOutOfBounds));
        }

        if len_after > value.len() {
            value.resize(len_after, 0);
        }

        value[offset..len_after]
            .copy_from_slice(self.memory.as_mut().read(src_ptr, write_len)?);

        let contract_id = self.internal_contract()?;
        self.storage_write_slot(contract_id, key, value)
    }

    /// Implementation of SRDD/SRDI opcodes
    pub(crate) fn dynamic_storage_read(
        &mut self,
        buffer_ptr: u64,
        key_ptr: u64,
        offset: u64,
        len: u64,
    ) -> IoResult<(), S::DataError> {
        let contract_id = self.internal_contract()?;
        let key = Bytes32::from(self.memory().read_bytes(key_ptr)?);
        self.storage_read_to_memory(contract_id, key, buffer_ptr, offset, len)?;
        Ok(())
    }

    /// Implementation of SWRD/SWRI opcodes
    pub(crate) fn dynamic_storage_write(
        &mut self,
        key_ptr: u64,
        value_ptr: u64,
        len: u64,
    ) -> IoResult<(), S::DataError> {
        let contract_id = self.internal_contract()?;
        let key = Bytes32::from(self.memory().read_bytes(key_ptr)?);
        self.storage_write_from_memory(contract_id, key, value_ptr, len)?;
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
        let contract_id = self.internal_contract()?;
        let key = Bytes32::from(self.memory().read_bytes(key_ptr)?);
        self.storage_update_from_memory(contract_id, key, value_ptr, offset, len)?;
        Ok(())
    }

    /// Implementation of SPLD opcode
    pub(crate) fn storage_preload(
        &mut self,
        r_dst_len: RegId,
        key: Bytes32,
    ) -> Result<(), RuntimeError<S::DataError>> {
        let contract_id = self.internal_contract()?;
        match self.storage_read_slot(contract_id, key, |_, v| v.map(|data| data.len()))? {
            Some(len) => {
                self.registers[RegId::ERR] = 0;
                self.write_user_register(r_dst_len, len as u64)?;
                Ok(())
            }
            None => {
                self.registers[RegId::ERR] = 1;
                self.write_user_register(r_dst_len, 0)?;
                Ok(())
            }
        }
    }
}

/// Returns an iterator over the keys in the range `[start_key, start_key + range)`.
/// If the range exceeds the maximum key, returns `None` for the keys that exceed the
/// maximum.
pub fn key_range(
    start_key: Bytes32,
    range: usize,
) -> impl Iterator<Item = Option<Bytes32>> {
    let start_key = primitive_types::U256::from_big_endian(&*start_key);

    (0..range).map(move |i| {
        start_key
            .checked_add(primitive_types::U256::from(i))
            .map(|key| Bytes32::new(key.into()))
    })
}

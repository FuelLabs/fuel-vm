use fuel_tx::{Bytes32, PanicReason};

use crate::{convert, error::RuntimeError, prelude::{Interpreter, Memory}, storage::InterpreterStorage};

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
{

    /// Returns `Ok(true)` if the storage slot identified by `key` was found, and `Ok(false)` otherwise.
    /// Errors if the read goes out of bounds of the stored value.
    pub(crate) fn storage_read_to_memory(&mut self, key: Bytes32, dst_ptr: u64, offset: u64, len: u64) -> Result<bool, RuntimeError<S::DataError>>
    {
        let offset = convert::to_usize(offset).ok_or(PanicReason::MemoryOverflow)?;

        let len = convert::to_usize(len).ok_or(PanicReason::MemoryOverflow)?;

        let contract_id = self.internal_contract()?;
        let owner = self.ownership_registers();

        let dst = self.memory.as_mut().write(owner, dst_ptr, len)?;
        let value = self.storage.contract_state(
            &contract_id,
            &key
        ).map_err(RuntimeError::Storage)?;

        let Some(value) = value else {
            return Ok(false);
        };

        let value = value.as_ref().as_ref();

        let end = offset.saturating_add(len);
        if end >= value.len() {
            // attempting to read past the end of the stored value
            return Err(RuntimeError::Recoverable(PanicReason::StorageOutOfBounds));
        }

        dst.copy_from_slice(&value[offset..end]);

        Ok(true)
    }

    /// Preloads the storage slot identified by `key` into a special memory area, returning its size.
    /// Returns `Ok(None)` if the slot is not found.
    pub(crate) fn storage_preload(&mut self, key: Bytes32) -> Result<Option<u64>, RuntimeError<S::DataError>> {
        let contract_id = self.internal_contract()?;
        let value = self.storage.contract_state(
            &contract_id,
            &key
        ).map_err(RuntimeError::Storage)?;

        let Some(value) = value else {
            return Ok(None);
        };

        let dst = self.memory.as_mut().storage_preload_mut();
        *dst = value.as_ref().as_ref().to_vec();
        Ok(Some(dst.len() as u64))
    }
}
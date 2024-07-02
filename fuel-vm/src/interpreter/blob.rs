use fuel_asm::{
    RegisterId,
    Word,
};
use fuel_storage::StorageSize;
use fuel_tx::PanicReason;
use fuel_types::BlobId;

use crate::{
    error::IoResult,
    prelude::*,
    storage::{
        BlobData,
        InterpreterStorage,
    },
};

use super::{
    internal::inc_pc,
    split_registers,
    GetRegMut,
    Interpreter,
    Memory,
    SystemRegisters,
    WriteRegKey,
};

impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal>
where
    M: Memory,
    S: InterpreterStorage,
    <S as InterpreterStorage>::DataError: From<S::DataError>,
{
    pub(crate) fn blob_size(
        &mut self,
        dst: RegisterId,
        blob_id_ptr: Word,
    ) -> IoResult<(), S::DataError> {
        self.gas_charge(self.interpreter_params.gas_costs.bsiz().base())?;

        let blob_id = BlobId::from(self.memory.as_ref().read_bytes(blob_id_ptr)?);

        let Some(size) =
            <S as StorageSize<BlobData>>::size_of_value(&self.storage, &blob_id)
                .map_err(RuntimeError::Storage)?
        else {
            return Err(PanicReason::BlobNotFound.into());
        };
        self.dependent_gas_charge_without_base(
            self.interpreter_params.gas_costs.bsiz(),
            size as Word,
        )?;
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(dst)?];
        *result = size as Word;
        Ok(inc_pc(pc)?)
    }

    pub(crate) fn blob_load_data(
        &mut self,
        dst_ptr: Word,
        blob_id_ptr: Word,
        blob_offset: Word,
        len: Word,
    ) -> IoResult<(), S::DataError> {
        self.dependent_gas_charge(self.interpreter_params.gas_costs.bldd(), len)?;

        let blob_offset: usize = blob_offset
            .try_into()
            .map_err(|_| PanicReason::MemoryOverflow)?;

        let blob_id = BlobId::from(self.memory.as_ref().read_bytes(blob_id_ptr)?);
        let owner = self.ownership_registers();
        let dst = self.memory.as_mut().write(owner, dst_ptr, len)?;

        let blob = <S as StorageInspect<BlobData>>::get(&self.storage, &blob_id)
            .map_err(RuntimeError::Storage)?
            .ok_or(PanicReason::BlobNotFound)?;
        let blob = blob.as_ref().as_ref();

        let end = blob_offset.saturating_add(dst.len()).min(blob.len());
        let data = blob.get(blob_offset..end).unwrap_or_default();

        dst[..data.len()].copy_from_slice(data);
        dst[data.len()..].fill(0);

        Ok(inc_pc(self.registers.pc_mut())?)
    }
}

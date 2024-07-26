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
    memory::copy_from_slice_zero_fill,
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
        let gas_cost = self
            .interpreter_params
            .gas_costs
            .bsiz()
            .map_err(PanicReason::from)?;
        self.gas_charge(gas_cost.base())?;

        let blob_id = BlobId::from(self.memory.as_ref().read_bytes(blob_id_ptr)?);

        let size = <S as StorageSize<BlobData>>::size_of_value(&self.storage, &blob_id)
            .map_err(RuntimeError::Storage)?
            .ok_or(PanicReason::BlobNotFound)?;

        self.dependent_gas_charge_without_base(gas_cost, size as Word)?;
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
        let gas_cost = self
            .interpreter_params
            .gas_costs
            .bldd()
            .map_err(PanicReason::from)?;
        self.gas_charge(gas_cost.base())?;

        let blob_id = BlobId::from(self.memory.as_ref().read_bytes(blob_id_ptr)?);
        let owner = self.ownership_registers();

        let size = <S as StorageSize<BlobData>>::size_of_value(&self.storage, &blob_id)
            .map_err(RuntimeError::Storage)?
            .ok_or(PanicReason::BlobNotFound)?;
        self.dependent_gas_charge_without_base(gas_cost, len.max(size as Word))?;

        let blob = <S as StorageInspect<BlobData>>::get(&self.storage, &blob_id)
            .map_err(RuntimeError::Storage)?
            .ok_or(PanicReason::BlobNotFound)?;
        let blob = blob.as_ref().as_ref();

        copy_from_slice_zero_fill(
            self.memory.as_mut(),
            owner,
            blob,
            dst_ptr,
            blob_offset,
            len,
        )?;

        Ok(inc_pc(self.registers.pc_mut())?)
    }
}

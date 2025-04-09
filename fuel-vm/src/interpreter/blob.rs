use fuel_asm::{
    RegId,
    Word,
};
use fuel_tx::PanicReason;
use fuel_types::BlobId;

use crate::{
    error::IoResult,
    interpreter::{
        contract::blob_size,
        memory::copy_from_storage_zero_fill,
    },
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

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
    <S as InterpreterStorage>::DataError: From<S::DataError>,
{
    pub(crate) fn blob_size(
        &mut self,
        dst: RegId,
        blob_id_ptr: Word,
    ) -> IoResult<(), S::DataError> {
        let gas_cost = self
            .interpreter_params
            .gas_costs
            .bsiz()
            .map_err(PanicReason::from)?;
        self.gas_charge(gas_cost.base())?;

        let blob_id = BlobId::from(self.memory.as_ref().read_bytes(blob_id_ptr)?);

        let size = blob_size(&self.storage, &blob_id)?;

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

        let blob_len = blob_size(&self.storage, &blob_id)?;
        let charge_len = len.max(blob_len as Word);
        self.dependent_gas_charge_without_base(gas_cost, charge_len)?;

        copy_from_storage_zero_fill::<BlobData, _>(
            self.memory.as_mut(),
            owner,
            &self.storage,
            dst_ptr,
            len,
            &blob_id,
            blob_offset,
            blob_len,
            PanicReason::BlobNotFound,
        )?;

        Ok(inc_pc(self.registers.pc_mut())?)
    }
}

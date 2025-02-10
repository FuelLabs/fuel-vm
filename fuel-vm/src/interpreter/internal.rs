use super::{
    ExecutableTransaction,
    Interpreter,
    Memory,
    MemoryInstance,
    RuntimeBalances,
};
use crate::{
    constraints::reg_key::*,
    context::Context,
    error::SimpleResult,
};

use fuel_asm::{
    Flags,
    Instruction,
    PanicReason,
};
use fuel_tx::{
    field::Outputs,
    Output,
};
use fuel_types::{
    canonical::Serialize,
    AssetId,
    BlockHeight,
    Bytes32,
    ContractId,
    Word,
};

use core::ops::Range;

#[cfg(test)]
mod message_tests;
#[cfg(test)]
mod tests;

impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal>
where
    M: Memory,
    Tx: ExecutableTransaction,
{
    pub(crate) fn update_memory_output(&mut self, idx: usize) -> SimpleResult<()> {
        let tx_offset = self.tx_offset();
        update_memory_output(&self.tx, self.memory.as_mut(), tx_offset, idx)
    }
}

/// Increase the variable output with a given asset ID. Modifies both the referenced tx
/// and the serialized tx in vm memory.
pub(crate) fn set_variable_output<Tx: ExecutableTransaction>(
    tx: &mut Tx,
    memory: &mut MemoryInstance,
    tx_offset: usize,
    idx: usize,
    variable: Output,
) -> SimpleResult<()> {
    tx.replace_variable_output(idx, variable)?;
    update_memory_output(tx, memory, tx_offset, idx)
}

fn absolute_output_offset<Tx: Outputs>(
    tx: &Tx,
    tx_offset: usize,
    idx: usize,
) -> Option<usize> {
    tx.outputs_offset_at(idx)
        .map(|offset| tx_offset.saturating_add(offset))
}

pub(crate) fn absolute_output_mem_range<Tx: Outputs>(
    tx: &Tx,
    tx_offset: usize,
    idx: usize,
) -> Option<Range<usize>> {
    let offset = absolute_output_offset(tx, tx_offset, idx)?;
    let size = tx.outputs().get(idx)?.size();
    Some(offset..offset.saturating_add(size))
}

pub(crate) fn update_memory_output<Tx: ExecutableTransaction>(
    tx: &Tx,
    memory: &mut MemoryInstance,
    tx_offset: usize,
    idx: usize,
) -> SimpleResult<()> {
    let range = absolute_output_mem_range(tx, tx_offset, idx)
        .ok_or(PanicReason::OutputNotFound)?;
    let mut mem = memory.write_noownerchecks(range.start, range.len())?;
    let output = tx
        .outputs()
        .get(idx)
        .expect("Invalid output index; checked above");
    output
        .encode(&mut mem)
        .expect("Unable to write output into given memory range");
    Ok(())
}

impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal>
where
    M: Memory,
{
    pub(crate) fn set_flag(&mut self, a: Word) -> SimpleResult<()> {
        let (SystemRegisters { flag, pc, .. }, _) = split_registers(&mut self.registers);
        set_flag(flag, pc, a)
    }

    pub(crate) const fn context(&self) -> &Context {
        &self.context
    }

    pub(crate) const fn is_predicate(&self) -> bool {
        matches!(
            self.context,
            Context::PredicateEstimation { .. } | Context::PredicateVerification { .. }
        )
    }

    pub(crate) fn internal_contract(&self) -> Result<ContractId, PanicReason> {
        internal_contract(&self.context, self.registers.fp(), self.memory.as_ref())
    }

    pub(crate) fn get_block_height(&self) -> Result<BlockHeight, PanicReason> {
        self.context()
            .block_height()
            .ok_or(PanicReason::TransactionValidity)
    }
}

pub(crate) fn clear_err(mut err: RegMut<ERR>) {
    *err = 0;
}

pub(crate) fn set_err(mut err: RegMut<ERR>) {
    *err = 1;
}

pub(crate) fn set_flag(
    mut flag: RegMut<FLAG>,
    pc: RegMut<PC>,
    a: Word,
) -> SimpleResult<()> {
    let Some(flags) = Flags::from_bits(a) else {
        return Err(PanicReason::InvalidFlags.into())
    };

    *flag = flags.bits();

    Ok(inc_pc(pc)?)
}

pub(crate) fn inc_pc(mut pc: RegMut<PC>) -> Result<(), PanicReason> {
    pc.checked_add(Instruction::SIZE as Word)
        .ok_or(PanicReason::MemoryOverflow)
        .map(|i| *pc = i)
}

pub(crate) fn tx_id(memory: &MemoryInstance) -> Bytes32 {
    Bytes32::new(memory.read_bytes(0u64).expect("Bytes32::LEN < MEM_SIZE"))
}

/// Reduces the unspent balance of the base asset
pub(crate) fn base_asset_balance_sub(
    base_asset_id: &AssetId,
    balances: &mut RuntimeBalances,
    memory: &mut MemoryInstance,
    value: Word,
) -> SimpleResult<()> {
    external_asset_id_balance_sub(balances, memory, base_asset_id, value)
}

/// Reduces the unspent balance of a given asset ID
pub(crate) fn external_asset_id_balance_sub(
    balances: &mut RuntimeBalances,
    memory: &mut MemoryInstance,
    asset_id: &AssetId,
    value: Word,
) -> SimpleResult<()> {
    balances
        .checked_balance_sub(memory, asset_id, value)
        .ok_or(PanicReason::NotEnoughBalance)?;

    Ok(())
}

pub(crate) fn current_contract(
    context: &Context,
    fp: Reg<FP>,
    memory: &MemoryInstance,
) -> Result<Option<ContractId>, PanicReason> {
    if context.is_internal() {
        Ok(Some(ContractId::new(memory.read_bytes(*fp)?)))
    } else {
        Ok(None)
    }
}

pub(crate) fn internal_contract(
    context: &Context,
    fp: Reg<FP>,
    memory: &MemoryInstance,
) -> Result<ContractId, PanicReason> {
    current_contract(context, fp, memory)?.ok_or(PanicReason::ExpectedInternalContext)
}

pub(crate) fn set_frame_pointer(
    context: &mut Context,
    mut register: RegMut<FP>,
    fp: Word,
) {
    context.update_from_frame_pointer(fp);

    *register = fp;
}

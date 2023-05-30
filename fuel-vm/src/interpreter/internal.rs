use super::{receipts::ReceiptsCtx, ExecutableTransaction, Interpreter, RuntimeBalances};
use super::{MemoryRange, VmMemory};
use crate::constraints::reg_key::*;
use crate::context::Context;
use crate::error::RuntimeError;

use fuel_asm::{Flags, Instruction, PanicReason, RegId};
use fuel_tx::field::{Outputs, ReceiptsRoot};
use fuel_tx::Script;
use fuel_tx::{Output, Receipt};
use fuel_types::bytes::SizedBytes;
use fuel_types::{AssetId, BlockHeight, Bytes32, ContractId, Word};

use core::mem;

#[cfg(test)]
mod message_tests;
#[cfg(all(test, feature = "random"))]
mod tests;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn update_memory_output(&mut self, idx: usize) -> Result<(), RuntimeError> {
        update_memory_output(&mut self.tx, &mut self.memory, self.params.tx_offset(), idx)
    }

    pub(crate) fn append_receipt(&mut self, receipt: Receipt) {
        append_receipt(
            AppendReceipt {
                receipts: &mut self.receipts,
                script: self.tx.as_script_mut(),
                tx_offset: self.params.tx_offset(),
                memory: &mut self.memory,
            },
            receipt,
        )
    }
}

/// Increase the variable output with a given asset ID. Modifies both the referenced tx and the
/// serialized tx in vm memory.
pub(crate) fn set_variable_output<Tx: ExecutableTransaction>(
    tx: &mut Tx,
    memory: &mut VmMemory,
    tx_offset: usize,
    idx: usize,
    variable: Output,
) -> Result<(), RuntimeError> {
    tx.replace_variable_output(idx, variable)?;
    update_memory_output(tx, memory, tx_offset, idx)
}

fn absolute_output_offset<Tx: Outputs>(tx: &Tx, tx_offset: usize, idx: usize) -> Option<usize> {
    tx.outputs_offset_at(idx).map(|offset| tx_offset + offset)
}

pub(crate) fn update_memory_output<Tx: ExecutableTransaction>(
    tx: &mut Tx,
    memory: &mut VmMemory,
    tx_offset: usize,
    idx: usize,
) -> Result<(), RuntimeError> {
    let range = absolute_output_offset(tx, tx_offset, idx)
        .and_then(|offset| {
            tx.outputs()
                .get(idx)
                .map(|output| MemoryRange::try_new_usize(offset, output.serialized_size()))
        })
        .ok_or(PanicReason::OutputNotFound)??;

    let mem = memory.force_mut_range(range);
    tx.output_to_mem(idx, mem)?;

    Ok(())
}

pub(crate) struct AppendReceipt<'vm> {
    pub receipts: &'vm mut ReceiptsCtx,
    pub script: Option<&'vm mut Script>,
    pub tx_offset: usize,
    pub memory: &'vm mut VmMemory,
}

pub(crate) fn append_receipt(input: AppendReceipt, receipt: Receipt) {
    let AppendReceipt {
        receipts,
        script,
        tx_offset,
        memory,
    } = input;
    receipts.push(receipt);

    if let Some(script) = script {
        let offset = tx_offset + script.receipts_root_offset();

        // TODO this generates logarithmic gas cost to the receipts count. This won't fit the
        // linear monadic model and should be discussed. Maybe the receipts tree should have
        // constant capacity so the gas cost is also constant to the maximum depth?
        let root = receipts.root();
        *script.receipts_root_mut() = root;

        // Transaction memory space length is already checked on initialization so its
        // guaranteed to fit
        memory.force_write_bytes(offset, &*root);
    }
}

impl<S, Tx> Interpreter<S, Tx> {
    /// Updates the boundaries of allocated memory.
    pub(crate) fn update_allocations(&mut self) -> Result<(), RuntimeError> {
        let sp = self.registers[RegId::SSP].max(self.registers[RegId::SP]);
        let hp = self.registers[RegId::HP];
        let _new_pages = self
            .memory
            .update_allocations(sp, hp)
            .map_err(|_| PanicReason::OutOfMemory)?;
        // TODO: gas cost for new pages
        Ok(())
    }

    /// Reserves `len` bytes of stack space. Returns the previous $ssp value,
    /// i.e. the first byte of the recently reserved memory area.
    pub(crate) fn reserve_stack(&mut self, len: Word) -> Result<Word, RuntimeError> {
        let (ssp, overflow) = self.registers[RegId::SSP].overflowing_add(len);

        if overflow || !self.is_external_context() && ssp > self.registers[RegId::SP] {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            let prev_ssp = mem::replace(&mut self.registers[RegId::SSP], ssp);
            self.update_allocations()?;
            Ok(prev_ssp)
        }
    }

    pub(crate) fn push_stack(&mut self, data: &[u8]) -> Result<(), RuntimeError> {
        let ssp = self.reserve_stack(data.len() as Word)?;

        debug_assert_eq!((self.registers[RegId::SSP] - ssp) as usize, data.len());
        self.memory.force_write_slice(ssp as usize, data);

        Ok(())
    }

    pub(crate) fn set_flag(&mut self, a: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { flag, pc, .. }, _) = split_registers(&mut self.registers);
        set_flag(flag, pc, a)
    }

    pub(crate) const fn context(&self) -> &Context {
        &self.context
    }

    pub(crate) const fn is_external_context(&self) -> bool {
        self.context().is_external()
    }

    pub(crate) const fn is_predicate(&self) -> bool {
        matches!(self.context, Context::Predicate { .. })
    }

    pub(crate) fn internal_contract(&self) -> Result<ContractId, RuntimeError> {
        internal_contract(&self.context, self.registers.fp(), &self.memory)
    }

    pub(crate) fn internal_contract_or_default(&self) -> ContractId {
        internal_contract_or_default(&self.context, self.registers.fp(), &self.memory)
    }

    pub(crate) const fn tx_offset(&self) -> usize {
        self.params().tx_offset()
    }

    pub(crate) fn get_block_height(&self) -> Result<BlockHeight, PanicReason> {
        self.context().block_height().ok_or(PanicReason::TransactionValidity)
    }
}

pub(crate) fn clear_err(mut err: RegMut<ERR>) {
    *err = 0;
}

pub(crate) fn set_err(mut err: RegMut<ERR>) {
    *err = 1;
}

pub(crate) fn set_flag(mut flag: RegMut<FLAG>, pc: RegMut<PC>, a: Word) -> Result<(), RuntimeError> {
    let Some(flags) = Flags::from_bits(a) else { return Err(PanicReason::ErrorFlag.into()) };

    *flag = flags.bits();

    inc_pc(pc)
}

pub(crate) fn inc_pc(mut pc: RegMut<PC>) -> Result<(), RuntimeError> {
    pc.checked_add(Instruction::SIZE as Word)
        .ok_or_else(|| PanicReason::ArithmeticOverflow.into())
        .map(|i| *pc = i)
}

pub(crate) fn tx_id(memory: &VmMemory) -> Bytes32 {
    // Safety: vm parameters guarantees enough space for txid
    Bytes32::from(memory.read_bytes(0).expect("Unreachable! Not enough memory for txid"))
}

/// Reduces the unspent balance of the base asset
pub(crate) fn base_asset_balance_sub(
    balances: &mut RuntimeBalances,
    memory: &mut VmMemory,
    value: Word,
) -> Result<(), RuntimeError> {
    external_asset_id_balance_sub(balances, memory, &AssetId::zeroed(), value)
}

/// Reduces the unspent balance of a given asset ID
pub(crate) fn external_asset_id_balance_sub(
    balances: &mut RuntimeBalances,
    memory: &mut VmMemory,
    asset_id: &AssetId,
    value: Word,
) -> Result<(), RuntimeError> {
    balances
        .checked_balance_sub(memory, asset_id, value)
        .ok_or(PanicReason::NotEnoughBalance)?;

    Ok(())
}

pub(crate) fn internal_contract_or_default(context: &Context, register: Reg<FP>, memory: &VmMemory) -> ContractId {
    internal_contract(context, register, memory).unwrap_or_default()
}

pub(crate) fn current_contract<'a>(
    context: &Context,
    fp: Reg<FP>,
    memory: &VmMemory,
) -> Result<Option<ContractId>, RuntimeError> {
    if context.is_internal() {
        Ok(Some(internal_contract(context, fp, memory)?))
    } else {
        Ok(None)
    }
}

pub(crate) fn internal_contract(
    context: &Context,
    register: Reg<FP>,
    memory: &VmMemory,
) -> Result<ContractId, RuntimeError> {
    let addr = internal_contract_addr(context, register)?;
    Ok(ContractId::from(memory.read_bytes(addr as usize)?))
}

pub(crate) fn internal_contract_addr(context: &Context, fp: Reg<FP>) -> Result<Word, RuntimeError> {
    if context.is_internal() {
        Ok(*fp) // TODO: use MemoryAddr type
    } else {
        Err(PanicReason::ExpectedInternalContext.into())
    }
}

pub(crate) fn set_frame_pointer(context: &mut Context, mut register: RegMut<FP>, fp: Word) {
    context.update_from_frame_pointer(fp);

    *register = fp;
}

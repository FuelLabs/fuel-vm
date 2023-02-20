use super::{ExecutableTransaction, Interpreter, RuntimeBalances};
use crate::constraints::reg_key::*;
use crate::constraints::CheckedMemRange;
use crate::consts::*;
use crate::context::Context;
use crate::crypto;
use crate::error::RuntimeError;

use fuel_asm::{Instruction, PanicReason, RegId};
use fuel_tx::field::Outputs;
use fuel_tx::field::ReceiptsRoot;
use fuel_tx::Script;
use fuel_tx::{Output, Receipt};
use fuel_types::bytes::SerializableVec;
use fuel_types::bytes::SizedBytes;
use fuel_types::{AssetId, Bytes32, ContractId, Word};

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
    memory: &mut [u8; VM_MEMORY_SIZE],
    tx_offset: usize,
    idx: usize,
    variable: Output,
) -> Result<(), RuntimeError> {
    tx.replace_variable_output(idx, variable)?;
    update_memory_output(tx, memory, tx_offset, idx)
}

pub(crate) fn set_message_output<Tx: ExecutableTransaction>(
    tx: &mut Tx,
    memory: &mut [u8; VM_MEMORY_SIZE],
    tx_offset: usize,
    idx: usize,
    message: Output,
) -> Result<(), RuntimeError> {
    tx.replace_message_output(idx, message)?;
    update_memory_output(tx, memory, tx_offset, idx)
}

fn absolute_output_offset<Tx: Outputs>(tx: &Tx, tx_offset: usize, idx: usize) -> Option<usize> {
    tx.outputs_offset_at(idx).map(|offset| tx_offset + offset)
}

pub(crate) fn absolute_output_mem_range<Tx: Outputs>(
    tx: &Tx,
    tx_offset: usize,
    idx: usize,
    memory_constraint: Option<core::ops::Range<Word>>,
) -> Result<Option<CheckedMemRange>, RuntimeError> {
    absolute_output_offset(tx, tx_offset, idx)
        .and_then(|offset| tx.outputs().get(idx).map(|output| (offset, output.serialized_size())))
        .map_or(Ok(None), |(offset, output_size)| match memory_constraint {
            Some(constraint) => Ok(Some(CheckedMemRange::new_with_constraint(
                offset as u64,
                output_size,
                constraint,
            )?)),
            None => Ok(Some(CheckedMemRange::new(offset as u64, output_size)?)),
        })
}

pub(crate) fn update_memory_output<Tx: ExecutableTransaction>(
    tx: &mut Tx,
    memory: &mut [u8; VM_MEMORY_SIZE],
    tx_offset: usize,
    idx: usize,
) -> Result<(), RuntimeError> {
    let mem_range = absolute_output_mem_range(tx, tx_offset, idx, None)?.ok_or(PanicReason::OutputNotFound)?;
    let mem = mem_range.write(memory);

    tx.output_to_mem(idx, mem)?;

    Ok(())
}

pub(crate) struct AppendReceipt<'vm> {
    pub receipts: &'vm mut Vec<Receipt>,
    pub script: Option<&'vm mut Script>,
    pub tx_offset: usize,
    pub memory: &'vm mut [u8; VM_MEMORY_SIZE],
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
        let root = crypto::ephemeral_merkle_root(receipts.iter().map(|r| r.clone().to_bytes()));

        *script.receipts_root_mut() = root;

        // Transaction memory space length is already checked on initialization so its
        // guaranteed to fit
        memory[offset..offset + Bytes32::LEN].copy_from_slice(&root[..]);
    }
}

impl<S, Tx> Interpreter<S, Tx> {
    pub(crate) fn reserve_stack(&mut self, len: Word) -> Result<Word, RuntimeError> {
        let (ssp, overflow) = self.registers[RegId::SSP].overflowing_add(len);

        if overflow || !self.is_external_context() && ssp > self.registers[RegId::SP] {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            Ok(mem::replace(&mut self.registers[RegId::SSP], ssp))
        }
    }

    pub(crate) fn push_stack(&mut self, data: &[u8]) -> Result<(), RuntimeError> {
        let ssp = self.reserve_stack(data.len() as Word)?;

        self.memory[ssp as usize..self.registers[RegId::SSP] as usize].copy_from_slice(data);

        Ok(())
    }

    pub(crate) fn set_flag(&mut self, a: Word) -> Result<(), RuntimeError> {
        let (ReadRegisters { flag, pc, .. }, _) = split_registers(&mut self.registers);
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

    pub(crate) fn internal_contract(&self) -> Result<&ContractId, RuntimeError> {
        internal_contract(&self.context, self.registers.fp(), &self.memory)
    }

    pub(crate) fn internal_contract_or_default(&self) -> ContractId {
        internal_contract_or_default(&self.context, self.registers.fp(), self.memory.as_ref())
    }

    pub(crate) const fn tx_offset(&self) -> usize {
        self.params().tx_offset()
    }

    pub(crate) fn block_height(&self) -> Result<u32, PanicReason> {
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
    *flag = a;

    inc_pc(pc)
}

pub(crate) fn inc_pc(mut pc: RegMut<PC>) -> Result<(), RuntimeError> {
    pc.checked_add(Instruction::SIZE as Word)
        .ok_or_else(|| PanicReason::ArithmeticOverflow.into())
        .map(|i| *pc = i)
}

pub(crate) fn tx_id(memory: &[u8; VM_MEMORY_SIZE]) -> &Bytes32 {
    // Safety: vm parameters guarantees enough space for txid
    unsafe { Bytes32::as_ref_unchecked(&memory[..Bytes32::LEN]) }
}

/// Reduces the unspent balance of the base asset
pub(crate) fn base_asset_balance_sub(
    balances: &mut RuntimeBalances,
    memory: &mut [u8; VM_MEMORY_SIZE],
    value: Word,
) -> Result<(), RuntimeError> {
    external_asset_id_balance_sub(balances, memory, &AssetId::zeroed(), value)
}

/// Reduces the unspent balance of a given asset ID
pub(crate) fn external_asset_id_balance_sub(
    balances: &mut RuntimeBalances,
    memory: &mut [u8; VM_MEMORY_SIZE],
    asset_id: &AssetId,
    value: Word,
) -> Result<(), RuntimeError> {
    balances
        .checked_balance_sub(memory, asset_id, value)
        .ok_or(PanicReason::NotEnoughBalance)?;

    Ok(())
}

pub(crate) fn internal_contract_or_default(
    context: &Context,
    register: Reg<FP>,
    memory: &[u8; VM_MEMORY_SIZE],
) -> ContractId {
    internal_contract(context, register, memory).map_or(Default::default(), |contract| *contract)
}

pub(crate) fn current_contract<'a>(
    context: &Context,
    fp: Reg<FP>,
    memory: &'a [u8; VM_MEMORY_SIZE],
) -> Result<Option<&'a ContractId>, RuntimeError> {
    if context.is_internal() {
        Ok(Some(internal_contract(context, fp, memory)?))
    } else {
        Ok(None)
    }
}

pub(crate) fn internal_contract<'a>(
    context: &Context,
    register: Reg<FP>,
    memory: &'a [u8; VM_MEMORY_SIZE],
) -> Result<&'a ContractId, RuntimeError> {
    let (c, _) = internal_contract_bounds(context, register)?;

    // Safety: Memory bounds logically verified by the interpreter
    let contract = unsafe {
        ContractId::as_ref_unchecked(CheckedMemRange::new_const::<{ ContractId::LEN }>(c as Word)?.read(memory))
    };

    Ok(contract)
}

pub(crate) fn internal_contract_bounds(context: &Context, fp: Reg<FP>) -> Result<(usize, usize), RuntimeError> {
    context
        .is_internal()
        .then(|| {
            let c = *fp as usize;
            let cx = c + ContractId::LEN;

            (c, cx)
        })
        .ok_or_else(|| PanicReason::ExpectedInternalContext.into())
}

pub(crate) fn set_frame_pointer(context: &mut Context, mut register: RegMut<FP>, fp: Word) {
    context.update_from_frame_pointer(fp);

    *register = fp;
}

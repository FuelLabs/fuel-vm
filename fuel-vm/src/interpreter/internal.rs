use super::{
    receipts::ReceiptsCtx,
    ExecutableTransaction,
    Interpreter,
    MemoryRange,
    RuntimeBalances,
};
use crate::{
    constraints::{
        reg_key::*,
        CheckedMemConstLen,
    },
    consts::*,
    context::Context,
    error::SimpleResult,
};

use fuel_asm::{
    Flags,
    Instruction,
    PanicReason,
    RegId,
};
use fuel_tx::{
    field::{
        Outputs,
        ReceiptsRoot,
    },
    Output,
    Receipt,
    Script,
};
use fuel_types::{
    canonical::{
        Serialize,
        SerializedSize,
    },
    AssetId,
    BlockHeight,
    Bytes32,
    ContractId,
    Word,
};

use core::mem;

#[cfg(test)]
mod message_tests;
#[cfg(test)]
mod tests;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn update_memory_output(&mut self, idx: usize) -> SimpleResult<()> {
        let tx_offset = self.tx_offset();
        update_memory_output(&mut self.tx, &mut self.memory, tx_offset, idx)
    }

    pub(crate) fn append_receipt(&mut self, receipt: Receipt) {
        let tx_offset = self.tx_offset();
        append_receipt(
            AppendReceipt {
                receipts: &mut self.receipts,
                script: self.tx.as_script_mut(),
                tx_offset,
                memory: &mut self.memory,
            },
            receipt,
        )
    }
}

/// Increase the variable output with a given asset ID. Modifies both the referenced tx
/// and the serialized tx in vm memory.
pub(crate) fn set_variable_output<Tx: ExecutableTransaction>(
    tx: &mut Tx,
    memory: &mut [u8; MEM_SIZE],
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
    tx.outputs_offset_at(idx).map(|offset| tx_offset + offset)
}

pub(crate) fn absolute_output_mem_range<Tx: Outputs>(
    tx: &Tx,
    tx_offset: usize,
    idx: usize,
) -> Result<Option<MemoryRange>, PanicReason> {
    absolute_output_offset(tx, tx_offset, idx)
        .and_then(|offset| tx.outputs().get(idx).map(|output| (offset, output.size())))
        .map_or(Ok(None), |(offset, output_size)| {
            Ok(Some(MemoryRange::new(offset, output_size)?))
        })
}

pub(crate) fn update_memory_output<Tx: ExecutableTransaction>(
    tx: &mut Tx,
    memory: &mut [u8; MEM_SIZE],
    tx_offset: usize,
    idx: usize,
) -> SimpleResult<()> {
    let mem_range = absolute_output_mem_range(tx, tx_offset, idx)?
        .ok_or(PanicReason::OutputNotFound)?;
    let mut mem = mem_range.write(memory);
    let output = tx
        .outputs_mut()
        .get_mut(idx)
        .expect("Invalid output index; checked above");
    output
        .encode(&mut mem)
        .expect("Unable to write output into given memory range");
    Ok(())
}

pub(crate) struct AppendReceipt<'vm> {
    pub receipts: &'vm mut ReceiptsCtx,
    pub script: Option<&'vm mut Script>,
    pub tx_offset: usize,
    pub memory: &'vm mut [u8; MEM_SIZE],
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

        // TODO this generates logarithmic gas cost to the receipts count. This won't fit
        // the linear monadic model and should be discussed. Maybe the receipts
        // tree should have constant capacity so the gas cost is also constant to
        // the maximum depth?
        let root = receipts.root();
        *script.receipts_root_mut() = root;

        // Transaction memory space length is already checked on initialization so its
        // guaranteed to fit
        memory[offset..offset + Bytes32::LEN].copy_from_slice(&root[..]);
    }
}

impl<S, Tx> Interpreter<S, Tx> {
    pub(crate) fn reserve_stack(&mut self, len: Word) -> Result<Word, PanicReason> {
        let (ssp, overflow) = self.registers[RegId::SSP].overflowing_add(len);

        if overflow || !self.is_external_context() && ssp > self.registers[RegId::SP] {
            Err(PanicReason::MemoryOverflow)
        } else {
            Ok(mem::replace(&mut self.registers[RegId::SSP], ssp))
        }
    }

    pub(crate) fn push_stack(&mut self, data: &[u8]) -> SimpleResult<()> {
        let ssp = self.reserve_stack(data.len() as Word)?;

        self.memory[ssp as usize..self.registers[RegId::SSP] as usize]
            .copy_from_slice(data);

        Ok(())
    }

    pub(crate) fn set_flag(&mut self, a: Word) -> SimpleResult<()> {
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
        matches!(
            self.context,
            Context::PredicateEstimation { .. } | Context::PredicateVerification { .. }
        )
    }

    pub(crate) fn internal_contract(&self) -> Result<&ContractId, PanicReason> {
        internal_contract(&self.context, self.registers.fp(), &self.memory)
    }

    pub(crate) fn internal_contract_or_default(&self) -> ContractId {
        internal_contract_or_default(
            &self.context,
            self.registers.fp(),
            self.memory.as_ref(),
        )
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
        return Err(PanicReason::ErrorFlag.into())
    };

    *flag = flags.bits();

    Ok(inc_pc(pc)?)
}

pub(crate) fn inc_pc(mut pc: RegMut<PC>) -> Result<(), PanicReason> {
    pc.checked_add(Instruction::SIZE as Word)
        .ok_or(PanicReason::MemoryOverflow)
        .map(|i| *pc = i)
}

pub(crate) fn tx_id(memory: &[u8; MEM_SIZE]) -> &Bytes32 {
    let memory = (&memory[..Bytes32::LEN])
        .try_into()
        .expect("Bytes32::LEN < MEM_SIZE");
    // Safety: vm parameters guarantees enough space for txid
    Bytes32::from_bytes_ref(memory)
}

/// Reduces the unspent balance of the base asset
pub(crate) fn base_asset_balance_sub(
    base_asset_id: &AssetId,
    balances: &mut RuntimeBalances,
    memory: &mut [u8; MEM_SIZE],
    value: Word,
) -> SimpleResult<()> {
    external_asset_id_balance_sub(balances, memory, base_asset_id, value)
}

/// Reduces the unspent balance of a given asset ID
pub(crate) fn external_asset_id_balance_sub(
    balances: &mut RuntimeBalances,
    memory: &mut [u8; MEM_SIZE],
    asset_id: &AssetId,
    value: Word,
) -> SimpleResult<()> {
    balances
        .checked_balance_sub(memory, asset_id, value)
        .ok_or(PanicReason::NotEnoughBalance)?;

    Ok(())
}

pub(crate) fn internal_contract_or_default(
    context: &Context,
    register: Reg<FP>,
    memory: &[u8; MEM_SIZE],
) -> ContractId {
    internal_contract(context, register, memory)
        .map_or(Default::default(), |contract| *contract)
}

pub(crate) fn current_contract<'a>(
    context: &Context,
    fp: Reg<FP>,
    memory: &'a [u8; MEM_SIZE],
) -> Result<Option<&'a ContractId>, PanicReason> {
    if context.is_internal() {
        Ok(Some(internal_contract(context, fp, memory)?))
    } else {
        Ok(None)
    }
}

pub(crate) fn internal_contract<'a>(
    context: &Context,
    register: Reg<FP>,
    memory: &'a [u8; MEM_SIZE],
) -> Result<&'a ContractId, PanicReason> {
    let range = internal_contract_bounds(context, register)?;

    // Safety: Memory bounds logically verified by the interpreter
    let contract = ContractId::from_bytes_ref(range.read(memory));

    Ok(contract)
}

pub(crate) fn internal_contract_bounds(
    context: &Context,
    fp: Reg<FP>,
) -> Result<CheckedMemConstLen<{ ContractId::LEN }>, PanicReason> {
    if context.is_internal() {
        CheckedMemConstLen::new(*fp)
    } else {
        Err(PanicReason::ExpectedInternalContext)
    }
}

pub(crate) fn set_frame_pointer(
    context: &mut Context,
    mut register: RegMut<FP>,
    fp: Word,
) {
    context.update_from_frame_pointer(fp);

    *register = fp;
}

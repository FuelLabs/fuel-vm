use super::{ExecutableTransaction, Interpreter, RuntimeBalances};
use super::{MemoryRange, VmMemory};
use crate::constraints::reg_key::*;
use crate::context::Context;
use crate::error::RuntimeError;

use fuel_asm::{Flags, Instruction, PanicReason, RegId};
use fuel_tx::field::{Outputs, ReceiptsRoot};
use fuel_tx::{Output, Receipt};
use fuel_types::bytes::SizedBytes;
use fuel_types::{AssetId, BlockHeight, Bytes32, ContractId, Word};

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
        self.receipts.push(receipt);

        if let Some(script) = self.tx.as_script_mut() {
            let offset = self.params.tx_offset() + script.receipts_root_offset();

            // TODO this generates logarithmic gas cost to the receipts count. This won't fit the
            // linear monadic model and should be discussed. Maybe the receipts tree should have
            // constant capacity so the gas cost is also constant to the maximum depth?
            let root = self.receipts.root();
            *script.receipts_root_mut() = root;

            // Transaction memory space length is already checked on initialization so its
            // guaranteed to fit
            self.memory.write_bytes(offset, &root);
        }
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
                .map(|output| MemoryRange::try_new(offset, output.serialized_size()))
        })
        .ok_or(PanicReason::OutputNotFound)??;

    let mem = memory.write(&range);
    tx.output_to_mem(idx, mem)?;

    Ok(())
}

impl<S, Tx> Interpreter<S, Tx> {
    /// Pushes given bytes to stack. Only to be used during initialization.
    pub(crate) fn init_push_stack(&mut self, data: &[u8]) {
        debug_assert!(
            self.registers[RegId::SSP] == self.registers[RegId::SP],
            "init_push_stack can only be used in during initialization"
        );

        let old_sp = self.registers[RegId::SP];
        let Some(new_sp) = self.registers[RegId::SP].checked_add(data.len() as Word) else {
            panic!("VM must have enough memory for initial allocations");
        };

        self.registers[RegId::SP] = new_sp;

        self.update_allocations()
            .expect("VM must have enough gas for initial allocations");

        self.mem_write_slice(old_sp, data)
            .expect("VM must have enough memory for initial allocations");

        self.registers[RegId::SSP] = new_sp;
    }

    pub(crate) fn set_flag(&mut self, a: Word) -> Result<(), RuntimeError> {
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

    pub(crate) fn internal_contract(&self) -> Result<ContractId, RuntimeError> {
        if self.context.is_internal() {
            Ok(ContractId::from(self.mem_read_bytes(self.registers[RegId::FP])?))
        } else {
            Err(PanicReason::ExpectedInternalContext.into())
        }
    }

    pub(crate) fn internal_contract_or_default(&self) -> ContractId {
        self.internal_contract().ok().unwrap_or_default()
    }

    pub(crate) fn current_contract(&self) -> Result<Option<ContractId>, RuntimeError> {
        if self.context.is_internal() {
            Ok(Some(self.internal_contract()?))
        } else {
            Ok(None)
        }
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
    Bytes32::from(memory.read_bytes(0))
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

// pub(crate) fn internal_contract_or_default(context: &Context, register: Reg<FP>, memory: &VmMemory) -> ContractId {
//     internal_contract(context, register, memory).unwrap_or_default()
// }

// pub(crate) fn current_contract(
//     context: &Context,
//     fp: Reg<FP>,
//     memory: &VmMemory,
// ) -> Result<Option<ContractId>, RuntimeError> {
//     if context.is_internal() {
//         Ok(Some(internal_contract(context, fp, memory)?))
//     } else {
//         Ok(None)
//     }
// }

// pub(crate) fn internal_contract(
//     context: &Context,
//     register: Reg<FP>,
//     memory: &VmMemory,
// ) -> Result<ContractId, RuntimeError> {
//     Ok(internal_contract_addr(context, register)?.read(memory))
// }

// pub(crate) fn internal_contract_addr(context: &Context, fp: Reg<FP>) -> Result<MemoryAddr, RuntimeError> {
//     if context.is_internal() {
//         Ok((*fp).to_raw_address())
//     } else {
//         Err(PanicReason::ExpectedInternalContext.into())
//     }
// }

pub(crate) fn set_frame_pointer(context: &mut Context, mut register: RegMut<FP>, fp: Word) {
    context.update_from_frame_pointer(fp);

    *register = fp;
}

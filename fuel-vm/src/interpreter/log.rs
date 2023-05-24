use super::{
    internal::{append_receipt, inc_pc, internal_contract_or_default, AppendReceipt},
    receipts::ReceiptsCtx,
    ExecutableTransaction, Interpreter, VmMemory,
};
use crate::{constraints::reg_key::*, consts::*};
use crate::{context::Context, error::RuntimeError};

use fuel_asm::PanicReason;
use fuel_crypto::Hasher;
use fuel_tx::{Receipt, Script};
use fuel_types::Word;

#[cfg(test)]
mod tests;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn log(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { fp, is, pc, .. }, _) = split_registers(&mut self.registers);
        let input = LogInput {
            memory: &mut self.memory,
            tx_offset: self.params.tx_offset(),
            context: &self.context,
            receipts: &mut self.receipts,
            script: self.tx.as_script_mut(),
            fp: fp.as_ref(),
            is: is.as_ref(),
            pc,
        };
        input.log(a, b, c, d)
    }

    pub(crate) fn log_data(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { fp, is, pc, .. }, _) = split_registers(&mut self.registers);
        let input = LogInput {
            memory: &mut self.memory,
            tx_offset: self.params.tx_offset(),
            context: &self.context,
            receipts: &mut self.receipts,
            script: self.tx.as_script_mut(),
            fp: fp.as_ref(),
            is: is.as_ref(),
            pc,
        };
        input.log_data(a, b, c, d)
    }
}

struct LogInput<'vm> {
    memory: &'vm mut VmMemory,
    tx_offset: usize,
    context: &'vm Context,
    receipts: &'vm mut ReceiptsCtx,
    script: Option<&'vm mut Script>,
    fp: Reg<'vm, FP>,
    is: Reg<'vm, IS>,
    pc: RegMut<'vm, PC>,
}

impl LogInput<'_> {
    pub(crate) fn log(self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let receipt = Receipt::log(
            internal_contract_or_default(self.context, self.fp, self.memory),
            a,
            b,
            c,
            d,
            *self.pc,
            *self.is,
        );

        append_receipt(
            AppendReceipt {
                receipts: self.receipts,
                script: self.script,
                tx_offset: self.tx_offset,
                memory: self.memory,
            },
            receipt,
        );

        inc_pc(self.pc)
    }

    pub(crate) fn log_data(self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        if d > MEM_MAX_ACCESS_SIZE || c > VM_MAX_RAM - d {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let cd = (c + d) as usize;
        let digest = Hasher::hash(&self.memory[c as usize..cd]);

        let receipt = Receipt::log_data_with_len(
            internal_contract_or_default(self.context, self.fp, self.memory),
            a,
            b,
            c,
            d,
            digest,
            self.memory[c as usize..cd].to_vec(),
            *self.pc,
            *self.is,
        );

        append_receipt(
            AppendReceipt {
                receipts: self.receipts,
                script: self.script,
                tx_offset: self.tx_offset,
                memory: self.memory,
            },
            receipt,
        );

        inc_pc(self.pc)
    }
}

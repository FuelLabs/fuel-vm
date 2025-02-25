use super::{
    internal::{
        inc_pc,
        internal_contract,
    },
    receipts::ReceiptsCtx,
    ExecutableTransaction,
    Interpreter,
    Memory,
    MemoryInstance,
};
use crate::{
    constraints::reg_key::*,
    context::Context,
    error::SimpleResult,
};

use fuel_tx::Receipt;
use fuel_types::Word;

#[cfg(test)]
mod tests;

impl<M, S, Tx, Ecal, OnVerifyError> Interpreter<M, S, Tx, Ecal, OnVerifyError>
where
    M: Memory,
    Tx: ExecutableTransaction,
{
    pub(crate) fn log(&mut self, a: Word, b: Word, c: Word, d: Word) -> SimpleResult<()> {
        let (SystemRegisters { fp, is, pc, .. }, _) =
            split_registers(&mut self.registers);
        let input = LogInput {
            memory: self.memory.as_mut(),
            context: &self.context,
            receipts: &mut self.receipts,
            fp: fp.as_ref(),
            is: is.as_ref(),
            pc,
        };
        input.log(a, b, c, d)
    }

    pub(crate) fn log_data(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
        d: Word,
    ) -> SimpleResult<()> {
        let (SystemRegisters { fp, is, pc, .. }, _) =
            split_registers(&mut self.registers);
        let input = LogInput {
            memory: self.memory.as_mut(),
            context: &self.context,
            receipts: &mut self.receipts,
            fp: fp.as_ref(),
            is: is.as_ref(),
            pc,
        };
        input.log_data(a, b, c, d)
    }
}

struct LogInput<'vm> {
    memory: &'vm MemoryInstance,
    context: &'vm Context,
    receipts: &'vm mut ReceiptsCtx,
    fp: Reg<'vm, FP>,
    is: Reg<'vm, IS>,
    pc: RegMut<'vm, PC>,
}

impl LogInput<'_> {
    pub(crate) fn log(self, a: Word, b: Word, c: Word, d: Word) -> SimpleResult<()> {
        let receipt = Receipt::log(
            internal_contract(self.context, self.fp, self.memory).unwrap_or_default(),
            a,
            b,
            c,
            d,
            *self.pc,
            *self.is,
        );

        self.receipts.push(receipt)?;

        Ok(inc_pc(self.pc)?)
    }

    pub(crate) fn log_data(self, a: Word, b: Word, c: Word, d: Word) -> SimpleResult<()> {
        let data = self.memory.read(c, d)?.to_vec();

        let receipt = Receipt::log_data(
            internal_contract(self.context, self.fp, self.memory).unwrap_or_default(),
            a,
            b,
            c,
            *self.pc,
            *self.is,
            data,
        );

        self.receipts.push(receipt)?;

        Ok(inc_pc(self.pc)?)
    }
}

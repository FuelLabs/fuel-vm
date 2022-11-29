use super::{ExecutableTransaction, Interpreter};
use crate::consts::*;
use crate::error::RuntimeError;

use fuel_asm::PanicReason;
use fuel_crypto::Hasher;
use fuel_tx::Receipt;
use fuel_types::Word;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn log(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let receipt = Receipt::log(
            self.internal_contract_or_default(),
            a,
            b,
            c,
            d,
            self.registers[REG_PC],
            self.registers[REG_IS],
        );

        self.append_receipt(receipt);

        self.inc_pc()
    }

    pub(crate) fn log_data(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        if d > MEM_MAX_ACCESS_SIZE || c >= VM_MAX_RAM - d {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let cd = (c + d) as usize;
        let digest = Hasher::hash(&self.memory[c as usize..cd]);

        let receipt = Receipt::log_data_with_len(
            self.internal_contract_or_default(),
            a,
            b,
            c,
            d,
            digest,
            self.memory[c as usize..cd].to_vec(),
            self.registers[REG_PC],
            self.registers[REG_IS],
        );

        self.append_receipt(receipt);

        self.inc_pc()
    }
}

use super::Interpreter;
use crate::consts::*;

use fuel_data::Word;
use fuel_tx::crypto::Hasher;
use fuel_tx::Receipt;

impl<S> Interpreter<S> {
    pub(crate) fn log(&mut self, a: Word, b: Word, c: Word, d: Word) -> bool {
        let receipt = Receipt::log(
            self.internal_contract_or_default(),
            a,
            b,
            c,
            d,
            self.registers[REG_PC],
            self.registers[REG_IS],
        );

        self.receipts.push(receipt);

        true
    }

    pub(crate) fn log_data(&mut self, a: Word, b: Word, c: Word, d: Word) -> bool {
        if d > MEM_MAX_ACCESS_SIZE || c >= VM_MAX_RAM - d {
            return false;
        }

        let cd = (c + d) as usize;
        let digest = Hasher::hash(&self.memory[c as usize..cd]);

        let receipt = Receipt::log_data(
            self.internal_contract_or_default(),
            a,
            b,
            c,
            d,
            digest,
            self.registers[REG_PC],
            self.registers[REG_IS],
        );

        self.receipts.push(receipt);

        true
    }
}

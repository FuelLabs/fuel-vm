use super::{ExecutableTransaction, Interpreter};
use crate::error::RuntimeError;

use fuel_asm::RegId;
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
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        );

        self.append_receipt(receipt);

        Ok(())
    }

    pub(crate) fn log_data(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let data = self.mem_read(c, d)?.to_vec();

        let digest = Hasher::hash(&data);

        let receipt = Receipt::log_data_with_len(
            self.internal_contract_or_default(),
            a,
            b,
            c,
            d,
            digest,
            data,
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        );

        self.append_receipt(receipt);

        Ok(())
    }
}

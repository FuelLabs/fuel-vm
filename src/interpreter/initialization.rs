use super::Interpreter;
use crate::consts::*;
use crate::context::Context;
use crate::error::InterpreterError;
use crate::interpreter::validation::CheckedTransaction;
use crate::storage::InterpreterStorage;

use fuel_tx::consts::MAX_INPUTS;
use fuel_types::bytes::{SerializableVec, SizedBytes};
use fuel_types::{AssetId, Word};
use itertools::Itertools;
use std::io;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn init(&mut self, checked_tx: CheckedTransaction) -> Result<(), InterpreterError> {
        let mut tx = checked_tx.tx;

        self.block_height = self.storage.block_height().map_err(InterpreterError::from_io)?;
        self.context = Context::from(&tx);

        self.frames.clear();
        self.receipts.clear();

        // Optimized for memset
        self.registers.iter_mut().for_each(|r| *r = 0);

        self.registers[REG_ONE] = 1;
        self.registers[REG_SSP] = 0;

        // Set heap area
        self.registers[REG_HP] = VM_MAX_RAM - 1;

        self.push_stack(tx.id().as_ref())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Set initial unused balances
        let free_balances = checked_tx.balances;
        // Clear balance indexes in case vm is being re-initialized
        self.unused_balance_index.clear();
        // Put free balances into vm memory
        for (asset_id, amount) in free_balances.iter().sorted_by_key(|i| i.0) {
            // push asset ID
            self.push_stack(asset_id.as_ref())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            // stack position
            let asset_id_offset = self.registers[REG_SSP] as usize;
            self.unused_balance_index.insert(*asset_id, asset_id_offset);
            // push spendable amount
            self.push_stack(&amount.to_be_bytes())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }
        // zero out remaining unused balance types
        for _i in free_balances.len()..(MAX_INPUTS as usize) {
            self.push_stack(&[0; AssetId::LEN + WORD_SIZE])
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }

        let tx_size = tx.serialized_size() as Word;
        self.registers[REG_GGAS] = tx.gas_limit();
        self.registers[REG_CGAS] = tx.gas_limit();

        self.push_stack(&tx_size.to_be_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.push_stack(tx.to_bytes().as_slice())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.registers[REG_SP] = self.registers[REG_SSP];

        self.tx = tx;

        Ok(())
    }
}

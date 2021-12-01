use super::Interpreter;
use crate::consts::*;
use crate::context::Context;
use crate::error::InterpreterError;
use crate::storage::InterpreterStorage;

use fuel_tx::consts::*;
use fuel_tx::{Input, Transaction};
use fuel_types::bytes::{SerializableVec, SizedBytes};
use fuel_types::{Color, Word};
use itertools::Itertools;

use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn init(&mut self, mut tx: Transaction) -> Result<(), InterpreterError> {
        tx.validate(self.block_height() as Word)?;
        tx.precompute_metadata();

        self.block_height = self
            .storage
            .block_height()
            .map_err(|e| InterpreterError::Initialization(e.into()))?;
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
            .map_err(InterpreterError::Initialization)?;

        let zeroes = &[0; MAX_INPUTS as usize * (Color::LEN + WORD_SIZE)];
        let ssp = self.registers[REG_SSP] as usize;

        self.push_stack(zeroes).map_err(InterpreterError::Initialization)?;

        if tx.is_script() {
            tx.inputs()
                .iter()
                .filter_map(|input| match input {
                    Input::Coin { color, amount, .. } => Some((color, amount)),
                    _ => None,
                })
                .sorted_by_key(|i| i.0)
                .take(MAX_INPUTS as usize)
                .fold(ssp, |mut ssp, (color, amount)| {
                    self.memory[ssp..ssp + Color::LEN].copy_from_slice(color.as_ref());
                    ssp += Color::LEN;

                    self.memory[ssp..ssp + WORD_SIZE].copy_from_slice(&amount.to_be_bytes());
                    ssp += WORD_SIZE;

                    ssp
                });
        }

        let tx_size = tx.serialized_size() as Word;

        if tx.is_script() {
            self.registers[REG_GGAS] = tx.gas_limit();
            self.registers[REG_CGAS] = tx.gas_limit();
        }

        self.push_stack(&tx_size.to_be_bytes())
            .map_err(InterpreterError::Initialization)?;
        self.push_stack(tx.to_bytes().as_slice())
            .map_err(InterpreterError::Initialization)?;

        self.registers[REG_SP] = self.registers[REG_SSP];

        self.tx = tx;

        Ok(())
    }
}

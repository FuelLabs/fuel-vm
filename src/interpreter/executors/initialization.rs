use crate::consts::*;
use crate::data::InterpreterStorage;
use crate::interpreter::{Context, ExecuteError, Interpreter};

use fuel_asm::Word;
use fuel_tx::bytes::{SerializableVec, SizedBytes};
use fuel_tx::consts::*;
use fuel_tx::{Color, Input, Transaction};
use itertools::Itertools;

use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn init(&mut self, mut tx: Transaction) -> Result<(), ExecuteError> {
        tx.validate(self.block_height() as Word)?;
        tx.precompute_metadata();

        self.context = Context::from(&tx);

        self.frames.clear();
        self.log.clear();

        // Optimized for memset
        self.registers.iter_mut().for_each(|r| *r = 0);

        self.registers[REG_ONE] = 1;
        self.registers[REG_SSP] = 0;

        // Set heap area
        self.registers[REG_HP] = VM_MAX_RAM - 1;

        self.push_stack(tx.id().as_ref())?;

        let zeroes = &[0; MAX_INPUTS as usize * (Color::size_of() + WORD_SIZE)];
        let ssp = self.registers[REG_SSP] as usize;
        self.push_stack(zeroes)?;

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
                    self.memory[ssp..ssp + Color::size_of()].copy_from_slice(color.as_ref());
                    ssp += Color::size_of();

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

        self.push_stack(&tx_size.to_be_bytes())?;
        self.push_stack(tx.to_bytes().as_slice())?;

        self.registers[REG_SP] = self.registers[REG_SSP];

        self.tx = tx;

        Ok(())
    }
}

use super::Interpreter;
use crate::consts::*;
use crate::context::Context;
use crate::error::InterpreterError;
use crate::storage::InterpreterStorage;

use fuel_asm::PanicReason;
use fuel_tx::consts::MAX_INPUTS;
use fuel_tx::{Input, Output, Transaction};
use fuel_types::bytes::{SerializableVec, SizedBytes};
use fuel_types::{Color, Word};
use itertools::Itertools;
use std::collections::HashMap;
use std::io;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn init(&mut self, mut tx: Transaction) -> Result<(), InterpreterError> {
        tx.validate(self.block_height() as Word)?;
        tx.precompute_metadata();

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
        let free_balances = Self::initial_free_balances(&mut tx)?;
        for (color, amount) in free_balances.iter().sorted_by_key(|i| i.0) {
            // push color
            self.push_stack(color.as_ref())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            // stack position
            let color_offset = self.registers[REG_SSP] as usize;
            self.unused_balance_index.insert(*color, color_offset);
            // push spendable amount
            self.push_stack(&amount.to_be_bytes())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }
        // zero out remaining unused balance types
        for _i in free_balances.len()..(MAX_INPUTS as usize) {
            self.push_stack(&[0; Color::LEN + WORD_SIZE])
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

    // compute the initial free balances for each asset type
    pub(crate) fn initial_free_balances(tx: &Transaction) -> Result<HashMap<Color, Word>, InterpreterError> {
        let mut balances = HashMap::<Color, Word>::new();

        // Add up all the inputs for each color
        for (color, amount) in tx.inputs().iter().filter_map(|input| match input {
            Input::Coin { color, amount, .. } => Some((color, amount)),
            _ => None,
        }) {
            *balances.entry(*color).or_default() += amount;
        }

        // Reduce by unavailable balances
        let base_asset = Color::default();
        if let Some(base_asset_balance) = balances.get_mut(&base_asset) {
            // remove byte costs from base asset spendable balance
            let byte_balance = (tx.metered_bytes_size() as Word) * tx.byte_price();
            *base_asset_balance = base_asset_balance
                .checked_sub(byte_balance)
                .ok_or(InterpreterError::Panic(PanicReason::NotEnoughBalance))?;
            // remove gas costs from base asset spendable balance
            *base_asset_balance = base_asset_balance
                .checked_sub(tx.gas_limit() * tx.gas_price())
                .ok_or(InterpreterError::Panic(PanicReason::NotEnoughBalance))?;
        }

        // reduce free balances by coin and withdrawal outputs
        for (color, amount) in tx.outputs().iter().filter_map(|output| match output {
            Output::Coin { color, amount, .. } => Some((color, amount)),
            Output::Withdrawal { color, amount, .. } => Some((color, amount)),
            _ => None,
        }) {
            let balance = balances.get_mut(&color).unwrap();
            *balance = balance
                .checked_sub(*amount)
                .ok_or(InterpreterError::Panic(PanicReason::NotEnoughBalance))?;
        }

        Ok(balances)
    }
}

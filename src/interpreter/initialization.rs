use super::Interpreter;
use crate::consts::*;
use crate::context::Context;
use crate::error::InterpreterError;
use crate::storage::InterpreterStorage;

use fuel_tx::{Input, Output, Transaction};
use fuel_types::bytes::{SerializableVec, SizedBytes};
use fuel_types::{Color, Word};

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

        // Set initial free balances
        self.free_balances = Self::initial_free_balances(&mut tx);

        self.push_stack(tx.id().as_ref())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let tx_size = tx.serialized_size() as Word;

        if let Transaction::Script { gas_limit, .. } = tx {
            self.registers[REG_GGAS] = gas_limit;
            self.registers[REG_CGAS] = gas_limit;
        }

        self.push_stack(&tx_size.to_be_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.push_stack(tx.to_bytes().as_slice())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.registers[REG_SP] = self.registers[REG_SSP];

        self.tx = tx;

        Ok(())
    }

    // compute the initial free balances for each asset type
    pub(crate) fn initial_free_balances(tx: &mut Transaction) -> HashMap<Color, Word> {
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
            *base_asset_balance -= byte_balance;
            // remove gas costs from base asset spendable balance
            if let (Some(gas_limit), Some(gas_price)) = (tx.gas_limit(), tx.gas_price()) {
                *base_asset_balance -= gas_limit * gas_price;
            }
        }

        // reduce free balances by coin and withdrawal outputs
        for (color, amount) in tx.outputs().iter().filter_map(|output| match output {
            Output::Coin { color, amount, .. } => Some((color, amount)),
            Output::Withdrawal { color, amount, .. } => Some((color, amount)),
            _ => None,
        }) {
            *balances.get_mut(&color).unwrap() -= amount;
        }

        balances
    }
}

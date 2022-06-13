use super::Interpreter;
use crate::consts::*;
use crate::context::Context;
use crate::error::InterpreterError;
use crate::storage::InterpreterStorage;

use fuel_tx::{Input, Output, Transaction, ValidationError};
use fuel_types::bytes::{SerializableVec, SizedBytes};
use fuel_types::{AssetId, Word};
use itertools::Itertools;

use std::collections::HashMap;
use std::io;

impl<S> Interpreter<S> {
    /// Initialize the VM with a given transaction
    pub fn init(&mut self, predicate: bool, block_height: u32, tx: Transaction) -> Result<(), InterpreterError> {
        self.tx = tx;

        self.tx
            .validate_without_signature(self.block_height() as Word, &self.params)?;
        self.tx.precompute_metadata();

        self.block_height = block_height;
        self.context = if predicate { Context::Predicate } else { Context::Script };

        self.frames.clear();
        self.receipts.clear();

        // Optimized for memset
        self.registers.iter_mut().for_each(|r| *r = 0);

        self.registers[REG_ONE] = 1;
        self.registers[REG_SSP] = 0;

        // Set heap area
        self.registers[REG_HP] = VM_MAX_RAM - 1;

        self.push_stack(self.tx.id().as_ref())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let free_balances = if predicate {
            // predicate verification should zero asset ids
            0
        } else {
            // Set initial unused balances
            let free_balances = self.initial_free_balances()?;

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

            free_balances.len() as Word
        };

        // zero out remaining unused balance types
        let unused_balances = self.params.max_inputs as Word - free_balances;
        let unused_balances = unused_balances * (AssetId::LEN + WORD_SIZE) as Word;

        // Its safe to just reserve since the memory was properly zeroed before in this routine
        self.reserve_stack(unused_balances)?;

        let tx_size = self.tx.serialized_size() as Word;

        self.registers[REG_GGAS] = self.tx.gas_limit();
        self.registers[REG_CGAS] = self.tx.gas_limit();

        self.push_stack(&tx_size.to_be_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let tx = self.tx.to_bytes();
        self.push_stack(tx.as_slice())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.registers[REG_SP] = self.registers[REG_SSP];

        Ok(())
    }

    // compute the initial free balances for each asset type
    pub(crate) fn initial_free_balances(&self) -> Result<HashMap<AssetId, Word>, InterpreterError> {
        let mut balances = HashMap::<AssetId, Word>::new();

        // Add up all the inputs for each asset ID
        for (asset_id, amount) in self.tx.inputs().iter().filter_map(|input| match input {
            Input::CoinPredicate { asset_id, amount, .. } | Input::CoinSigned { asset_id, amount, .. } => {
                Some((asset_id, amount))
            }
            _ => None,
        }) {
            *balances.entry(*asset_id).or_default() += amount;
        }

        // Reduce by unavailable balances
        let base_asset = AssetId::default();
        if let Some(base_asset_balance) = balances.get_mut(&base_asset) {
            // calculate the fee with used metered bytes + gas limit
            let factor = self.params.gas_price_factor as f64;

            let bytes = self
                .tx
                .byte_price()
                .saturating_mul(self.tx.metered_bytes_size() as Word);
            let bytes = (bytes as f64 / factor).ceil() as Word;

            let gas = self.tx.gas_price().saturating_mul(self.tx.gas_limit()) as f64;
            let gas = (gas / factor).ceil() as Word;

            let fee = bytes.checked_add(gas).ok_or(ValidationError::ArithmeticOverflow)?;

            // subtract total fee from base asset balance
            *base_asset_balance =
                base_asset_balance
                    .checked_sub(fee)
                    .ok_or(ValidationError::InsufficientFeeAmount {
                        expected: fee,
                        provided: *base_asset_balance,
                    })?;
        }

        // reduce free balances by coin and withdrawal outputs
        for (asset_id, amount) in self.tx.outputs().iter().filter_map(|output| match output {
            Output::Coin { asset_id, amount, .. } => Some((asset_id, amount)),
            Output::Withdrawal { asset_id, amount, .. } => Some((asset_id, amount)),
            _ => None,
        }) {
            let balance = balances
                .get_mut(asset_id)
                .ok_or(ValidationError::TransactionOutputCoinAssetIdNotFound(*asset_id))?;
            *balance = balance
                .checked_sub(*amount)
                .ok_or(ValidationError::InsufficientInputAmount {
                    asset: *asset_id,
                    expected: *amount,
                    provided: *balance,
                })?;
        }

        Ok(balances)
    }
}

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    /// Initialize the VM with a given transaction, backed by a storage provider that allows
    /// execution of contract opcodes.
    ///
    /// For predicate verification, check [`Self::init`]
    pub fn init_with_storage(&mut self, tx: Transaction) -> Result<(), InterpreterError> {
        let predicate = false;
        let block_height = self.storage.block_height().map_err(InterpreterError::from_io)?;

        self.init(predicate, block_height, tx)
    }
}

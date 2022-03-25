use crate::error::InterpreterError;
use crate::prelude::{Interpreter, InterpreterStorage};
use fuel_tx::{Output, Transaction};
use fuel_types::{AssetId, Word};

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    /// Finalize outputs post-execution.
    /// Set the appropriate change output values.
    /// Revert variable output amounts as needed
    /// TODO: do we want to set contract outputs in memory?
    pub(crate) fn finalize_outputs(&mut self, unused_gas_cost: Word, revert: bool) -> Result<(), InterpreterError> {
        let init_balances = Self::initial_free_balances(&self.tx)?;

        let mut update_outputs = match &self.tx {
            Transaction::Script { outputs, .. } => outputs.clone(),
            Transaction::Create { outputs, .. } => outputs.clone(),
        };

        // Update each output based on free balance
        for (idx, output) in update_outputs.iter_mut().enumerate() {
            if let Output::Change { asset_id, amount, .. } = output {
                let refund = if *asset_id == AssetId::default() {
                    unused_gas_cost
                } else {
                    0
                };

                // Safety: initialization verifies that every output has a compatible input asset
                // by returning a TransactionOutputChangeAssetIdNotFound or
                // TransactionOutputCoinAssetIdNotFound error for missing inputs.
                let final_amount = if revert {
                    init_balances[asset_id] + refund
                } else {
                    let balance = self.external_asset_id_balance(asset_id)?;
                    balance + refund
                };

                *amount = final_amount;
                self.set_output(idx, *output)?;
            }
            if let Output::Variable { amount, .. } = output {
                if revert {
                    // reset amounts to zero on revert
                    *amount = 0;
                    self.set_output(idx, *output)?;
                }
            }
        }

        Ok(())
    }
}

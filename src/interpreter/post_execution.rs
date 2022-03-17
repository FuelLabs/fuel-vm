use crate::error::InterpreterError;
use crate::prelude::{Interpreter, InterpreterStorage};
use fuel_tx::{Output, Transaction};
use fuel_types::{AssetId, Word};

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    /// Set the appropriate change output values after execution has concluded.
    pub(crate) fn update_change_amounts(
        &mut self,
        unused_gas_cost: Word,
        revert: bool,
    ) -> Result<(), InterpreterError> {
        let init_balances = Self::initial_free_balances(&self.tx)?;

        let update_outputs = match &self.tx {
            Transaction::Script { outputs, .. } => outputs.clone(),
            Transaction::Create { outputs, .. } => outputs.clone(),
        };

        // Update each output based on free balance
        for (idx, output) in update_outputs.iter().enumerate() {
            if let Output::Change { asset_id, .. } = output {
                let refund = if *asset_id == AssetId::default() {
                    unused_gas_cost
                } else {
                    0
                };

                let amount = if revert {
                    init_balances[asset_id] + refund
                } else {
                    let balance = self.external_asset_id_balance(asset_id)?;
                    balance + refund
                };
                self.set_change_output(idx, amount)?;
            }
            if let Output::Variable { .. } = output {
                if revert {
                    // reset amounts to zero on revert
                    self.revert_variable_output(idx)?;
                }
            }
        }

        Ok(())
    }
}

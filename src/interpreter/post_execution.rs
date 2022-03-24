use crate::error::InterpreterError;
use crate::prelude::{Interpreter, InterpreterStorage};
use fuel_tx::{Output, Transaction};
use fuel_types::{AssetId, Word};
use tracing::debug;

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

        let mut update_outputs = match &self.tx {
            Transaction::Script { outputs, .. } => outputs.clone(),
            Transaction::Create { outputs, .. } => outputs.clone(),
        };
        // Update each output based on free balance
        for output in update_outputs.iter_mut() {
            if let Output::Change { asset_id, amount, .. } = output {
                let refund = if *asset_id == AssetId::default() {
                    unused_gas_cost
                } else {
                    0
                };

                if revert {
                    *amount = init_balances[asset_id] + refund;
                } else {
                    let balance = self.external_asset_id_balance(asset_id)?;
                    *amount = balance + refund;
                }
            }
            if let Output::Variable { amount, .. } = output {
                if revert {
                    // reset amounts to zero on revert
                    *amount = 0;
                }
            }

            debug!("{:?}", output);
        }
        // set outputs on tx
        match &mut self.tx {
            Transaction::Script { outputs, .. } => *outputs = update_outputs,
            Transaction::Create { outputs, .. } => *outputs = update_outputs,
        }

        Ok(())
    }
}

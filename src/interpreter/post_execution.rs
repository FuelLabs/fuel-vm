use crate::error::InterpreterError;
use crate::prelude::{Interpreter, InterpreterStorage};
use fuel_tx::{Output, Transaction};
use fuel_types::{Color, Word};

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
        let init_balances = Self::initial_free_balances(&mut self.tx);

        // Update each output based on free balance
        let outputs = match &mut self.tx {
            Transaction::Script { outputs, .. } => outputs,
            Transaction::Create { outputs, .. } => outputs,
        };
        for output in outputs.iter_mut() {
            if let Output::Change { color, amount, .. } = output {
                let refund = if *color == Color::default() { unused_gas_cost } else { 0 };

                if revert {
                    *amount = init_balances[&color] + refund;
                } else {
                    *amount = self.external_color_balance(&color)? + refund;
                    // zero out free balance for this color
                    self.external_color_balance_sub(&color, *amount);
                }
            }
        }

        Ok(())
    }
}

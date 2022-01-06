use crate::error::InterpreterError;
use crate::prelude::{Interpreter, InterpreterStorage};
use fuel_tx::Output;
use fuel_types::Word;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub fn update_change_amounts(&mut self, unused_gas_cost: Word, revert: bool) -> Result<(), InterpreterError> {
        let init_balances = Self::initial_free_balances(&mut self.tx);

        // Update each output based on free balance
        for output in self.tx.outputs().iter_mut() {
            if let Output::Change { color, amount, .. } = output {
                let refund = if color == Default::default() {
                    unused_gas_cost
                } else {
                    0
                };

                if revert {
                    *amount = init_balances[&color] + refund;
                } else {
                    *amount = self.free_balances[&color] + refund;
                    *self.free_balances[&color] = 0;
                }
            }
        }

        Ok(())
    }
}

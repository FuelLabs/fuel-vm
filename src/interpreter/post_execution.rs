use crate::consts::*;
use crate::prelude::{ExecutableTransaction, Interpreter, InterpreterStorage, RuntimeError};

use std::io;

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    /// Finalize outputs post-execution.
    ///
    /// For more information, check [`ExecutableTransaction::update_outputs`].
    ///
    /// # Panics
    ///
    /// This will panic if the transaction is malformed (e.g. it contains an output change with
    /// asset id that doesn't exist as balance).
    ///
    /// The transaction validation is expected to halt in such case. Since the VM only accepts
    /// checked transactions - hence, validated - this case should be unreachable.
    pub(crate) fn finalize_outputs(&mut self, revert: bool) -> Result<(), RuntimeError> {
        let outputs = self.transaction().outputs().len();
        let params = &self.params;
        let tx = &mut self.tx;

        let remaining_gas = self.registers[REG_GGAS];

        tx.update_outputs(params, revert, remaining_gas, &self.initial_balances, &self.balances)
            .map_err(|e| io::Error::new(
                io::ErrorKind::Other,
                format!("a valid VM execution shouldn't result in a state where it can't compute its refund. This is a bug! {}", e)
            ))?;

        (0..outputs).try_for_each(|o| self.update_memory_output(o))
    }
}

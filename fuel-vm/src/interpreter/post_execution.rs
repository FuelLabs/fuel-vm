use crate::prelude::{
    ExecutableTransaction,
    Interpreter,
    InterpreterStorage,
    RuntimeError,
};

use crate::interpreter::{
    InitialBalances,
    RuntimeBalances,
};
use fuel_tx::FeeParameters;
use fuel_types::Word;
use std::io;

impl<S, T> Interpreter<S, T>
where
    S: InterpreterStorage,
{
    /// Finalize outputs post-execution.
    ///
    /// For more information, check [`ExecutableTransaction::update_outputs`].
    ///
    /// # Panics
    ///
    /// This will panic if the transaction is malformed (e.g. it contains an output change
    /// with asset id that doesn't exist as balance).
    ///
    /// The transaction validation is expected to halt in such case. Since the VM only
    /// accepts checked transactions - hence, validated - this case should be
    /// unreachable.
    pub(crate) fn finalize_outputs<Tx>(
        tx: &mut Tx,
        fee_params: &FeeParameters,
        revert: bool,
        remaining_gas: Word,
        initial_balances: &InitialBalances,
        balances: &RuntimeBalances,
    ) -> Result<(), RuntimeError>
    where
        Tx: ExecutableTransaction,
    {
        tx.update_outputs( revert, remaining_gas, initial_balances, balances, fee_params)
            .map_err(|e| io::Error::new(
                io::ErrorKind::Other,
                format!("a valid VM execution shouldn't result in a state where it can't compute its refund. This is a bug! {e}")
            ))?;

        Ok(())
    }
}

use crate::prelude::{
    Bug,
    BugVariant,
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
use fuel_types::{
    AssetId,
    Word,
};

impl<S, T, Ecal> Interpreter<S, T, Ecal>
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
        base_asset_id: &AssetId,
        revert: bool,
        remaining_gas: Word,
        initial_balances: &InitialBalances,
        balances: &RuntimeBalances,
    ) -> Result<(), RuntimeError<S::DataError>>
    where
        Tx: ExecutableTransaction,
    {
        tx.update_outputs(
            revert,
            remaining_gas,
            initial_balances,
            balances,
            fee_params,
            base_asset_id,
        )
        .map_err(|e| Bug::new(BugVariant::UncomputableRefund).with_message(e))?;

        Ok(())
    }
}

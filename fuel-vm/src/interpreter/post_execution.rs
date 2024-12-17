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
use fuel_tx::{
    FeeParameters,
    GasCosts,
};
use fuel_types::{
    AssetId,
    Word,
};

impl<M, S, T, Ecal, Trace> Interpreter<M, S, T, Ecal, Trace>
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
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn finalize_outputs<Tx>(
        tx: &mut Tx,
        gas_costs: &GasCosts,
        fee_params: &FeeParameters,
        base_asset_id: &AssetId,
        revert: bool,
        used_gas: Word,
        initial_balances: &InitialBalances,
        balances: &RuntimeBalances,
        gas_price: Word,
    ) -> Result<(), RuntimeError<S::DataError>>
    where
        Tx: ExecutableTransaction,
    {
        tx.update_outputs(
            revert,
            used_gas,
            initial_balances,
            balances,
            gas_costs,
            fee_params,
            base_asset_id,
            gas_price,
        )
        .map_err(|e| Bug::new(BugVariant::UncomputableRefund).with_message(e))?;

        Ok(())
    }
}

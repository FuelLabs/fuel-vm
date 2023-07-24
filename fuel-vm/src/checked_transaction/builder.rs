//! Extension trait for [`fuel_tx::TransactionBuilder`]

use super::{
    Checked,
    IntoChecked,
};
use crate::{
    checked_transaction::CheckPredicates,
    prelude::*,
};
use fuel_tx::ConsensusParams;
use fuel_types::BlockHeight;

/// Extension trait for [`fuel_tx::TransactionBuilder`] adding finalization methods
pub trait TransactionBuilderExt<Tx>
where
    Tx: IntoChecked,
{
    /// Finalize the builder into a [`Checked<Tx>`] of the correct type
    fn finalize_checked(
        &mut self,
        height: BlockHeight,
        gas_costs: GasCosts,
    ) -> Checked<Tx>;

    /// Finalize the builder into a [`Checked<Tx>`] of the correct type, with basic checks
    /// only
    fn finalize_checked_basic(&mut self, height: BlockHeight) -> Checked<Tx>;
}

impl<Tx: ExecutableTransaction> TransactionBuilderExt<Tx> for TransactionBuilder<Tx>
where
    Self: Finalizable<Tx>,
    Checked<Tx>: CheckPredicates,
{
    fn finalize_checked(
        &mut self,
        height: BlockHeight,
        gas_costs: GasCosts,
    ) -> Checked<Tx> {
        let tx_params = *self.get_tx_params();
        let predicate_params = *self.get_predicate_params();
        let script_params = *self.get_script_params();
        let contract_params = *self.get_contract_params();
        let fee_params = *self.get_fee_params();

        let consensus_params = ConsensusParams::new(
            tx_params,
            predicate_params,
            script_params,
            contract_params,
            fee_params,
        );
        let chain_id = *self.get_chain_id();
        self.finalize()
            .into_checked(height, &consensus_params, chain_id, gas_costs)
            .expect("failed to check tx")
    }

    fn finalize_checked_basic(&mut self, height: BlockHeight) -> Checked<Tx> {
        let tx_params = *self.get_tx_params();
        let predicate_params = *self.get_predicate_params();
        let script_params = *self.get_script_params();
        let contract_params = *self.get_contract_params();
        let fee_params = *self.get_fee_params();
        let consensus_params = ConsensusParams::new(
            tx_params,
            predicate_params,
            script_params,
            contract_params,
            fee_params,
        );
        let chain_id = *self.get_chain_id();
        self.finalize()
            .into_checked_basic(height, &consensus_params, &chain_id)
            .expect("failed to check tx")
    }
}

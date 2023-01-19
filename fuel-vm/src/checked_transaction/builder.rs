//! Extension trait for [`fuel_tx::TransactionBuilder`]

use super::{Checked, IntoChecked};
use crate::prelude::*;
use fuel_tx::ConsensusParameters;

/// Extension trait for [`fuel_tx::TransactionBuilder`] adding finalization methods
pub trait TransactionBuilderExt<Tx>
where
    Tx: IntoChecked,
{
    /// Finalize the builder into a [`Checked<Tx>`] of the correct type
    fn finalize_checked(&mut self, height: Word, params: &ConsensusParameters, gas_costs: GasCosts) -> Checked<Tx>;

    /// Finalize the builder into a [`Checked<Tx>`] of the correct type, with basic checks only
    fn finalize_checked_basic(&mut self, height: Word, params: &ConsensusParameters) -> Checked<Tx>;
}

impl<Tx: IntoChecked + ExecutableTransaction + fuel_tx::field::GasLimit> TransactionBuilderExt<Tx>
    for TransactionBuilder<Tx>
where
    Checked<Tx>: Clone,
    TransactionBuilder<Tx>: Finalizable<Tx>,
    <Tx as IntoChecked>::Metadata: crate::interpreter::CheckedMetadata,
{
    fn finalize_checked(&mut self, height: Word, params: &ConsensusParameters, gas_costs: GasCosts) -> Checked<Tx> {
        self.finalize()
            .into_checked(height, params, gas_costs)
            .expect("failed to check tx")
    }

    fn finalize_checked_basic(&mut self, height: Word, params: &ConsensusParameters) -> Checked<Tx> {
        self.finalize()
            .into_checked_basic(height, params)
            .expect("failed to check tx")
    }
}

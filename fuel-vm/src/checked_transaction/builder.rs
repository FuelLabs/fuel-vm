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

impl TransactionBuilderExt<Create> for TransactionBuilder<Create> {
    fn finalize_checked(&mut self, height: Word, params: &ConsensusParameters, gas_costs: GasCosts) -> Checked<Create> {
        self.finalize()
            .into_checked(height, params, gas_costs)
            .expect("failed to check tx")
    }

    fn finalize_checked_basic(&mut self, height: Word, params: &ConsensusParameters) -> Checked<Create> {
        self.finalize()
            .into_checked_basic(height, params)
            .expect("failed to check tx")
    }
}

impl TransactionBuilderExt<Mint> for TransactionBuilder<Mint> {
    fn finalize_checked(&mut self, height: Word, params: &ConsensusParameters, gas_costs: GasCosts) -> Checked<Mint> {
        self.finalize()
            .into_checked(height, params, gas_costs)
            .expect("failed to check tx")
    }

    fn finalize_checked_basic(&mut self, height: Word, params: &ConsensusParameters) -> Checked<Mint> {
        self.finalize()
            .into_checked_basic(height, params)
            .expect("failed to check tx")
    }
}

impl TransactionBuilderExt<Script> for TransactionBuilder<Script> {
    fn finalize_checked(&mut self, height: Word, params: &ConsensusParameters, gas_costs: GasCosts) -> Checked<Script> {
        self.finalize()
            .into_checked(height, params, gas_costs)
            .expect("failed to check tx")
    }

    fn finalize_checked_basic(&mut self, height: Word, params: &ConsensusParameters) -> Checked<Script> {
        self.finalize()
            .into_checked_basic(height, params)
            .expect("failed to check tx")
    }
}

//! Extension trait for [`fuel_tx::TransactionBuilder`]

use super::{Checked, IntoChecked};
use crate::{checked_transaction::CheckPredicates, prelude::*};
use fuel_tx::{Finalizable, TransactionBuilder};
use fuel_types::BlockHeight;

/// Extension trait for [`fuel_tx::TransactionBuilder`] adding finalization methods
pub trait TransactionBuilderExt<Tx>
where
    Tx: IntoChecked,
{
    /// Finalize the builder into a [`Checked<Tx>`] of the correct type
    fn finalize_checked(&mut self, height: BlockHeight, gas_price: u64) -> Checked<Tx>;

    /// Finalize the builder into a [`Checked<Tx>`] of the correct type, with basic checks
    /// only
    fn finalize_checked_basic(
        &mut self,
        height: BlockHeight,
        gas_price: u64,
    ) -> Checked<Tx>;
}

impl<Tx: ExecutableTransaction> TransactionBuilderExt<Tx> for TransactionBuilder<Tx>
where
    Self: Finalizable<Tx>,
    Checked<Tx>: CheckPredicates,
{
    fn finalize_checked(&mut self, height: BlockHeight, gas_price: u64) -> Checked<Tx> {
        self.finalize()
            .into_checked(height, self.get_params(), gas_price)
            .expect("failed to check tx")
    }

    fn finalize_checked_basic(
        &mut self,
        height: BlockHeight,
        gas_price: u64,
    ) -> Checked<Tx> {
        self.finalize()
            .into_checked_basic(height, self.get_params(), gas_price)
            .expect("failed to check tx")
    }
}

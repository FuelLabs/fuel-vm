//! Extension trait for [`fuel_tx::TransactionBuilder`]

use super::{
    IntoChecked,
    PartiallyCheckedTx,
};
use crate::{
    checked_transaction::CheckPredicates,
    prelude::*,
};
use fuel_tx::{
    Finalizable,
    TransactionBuilder,
};
use fuel_types::BlockHeight;

/// Extension trait for [`fuel_tx::TransactionBuilder`] adding finalization methods
pub trait TransactionBuilderExt<Tx>
where
    Tx: IntoChecked,
{
    /// Finalize the builder into a [`PartiallyCheckedTx<Tx>`] of the correct type
    fn finalize_partially_checked(&self, height: BlockHeight) -> PartiallyCheckedTx<Tx>;

    /// Finalize the builder into a [`PartiallyCheckedTx<Tx>`] of the correct type, with
    /// basic checks only
    fn finalize_partially_checked_basic(
        &self,
        height: BlockHeight,
    ) -> PartiallyCheckedTx<Tx>;
}

impl<Tx: ExecutableTransaction> TransactionBuilderExt<Tx> for TransactionBuilder<Tx>
where
    Self: Finalizable<Tx>,
    PartiallyCheckedTx<Tx>: CheckPredicates,
{
    fn finalize_partially_checked(&self, height: BlockHeight) -> PartiallyCheckedTx<Tx> {
        self.finalize()
            .into_partially_checked(height, self.get_params())
            .expect("failed to check tx")
    }

    fn finalize_partially_checked_basic(
        &self,
        height: BlockHeight,
    ) -> PartiallyCheckedTx<Tx> {
        self.finalize()
            .into_partially_checked_basic(height, self.get_params())
            .expect("failed to check tx")
    }
}

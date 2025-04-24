//! Extension trait for [`fuel_tx::TransactionBuilder`]

use super::{
    Checked,
    IntoChecked,
};
use crate::{
    checked_transaction::CheckPredicates,
    prelude::*,
    storage::predicate::PredicateStorageRequirements,
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
    /// Finalize the builder into a [`Checked<Tx>`] of the correct type
    fn finalize_checked(&self, height: BlockHeight) -> Checked<Tx>;

    /// Finalize the builder into a [`Checked<Tx>`] of the correct type
    /// using the storage during verification.
    fn finalize_checked_with_storage(
        &self,
        height: BlockHeight,
        storage: &impl PredicateStorageRequirements,
    ) -> Checked<Tx>;

    /// Finalize the builder into a [`Checked<Tx>`] of the correct type, with basic checks
    /// only
    fn finalize_checked_basic(&self, height: BlockHeight) -> Checked<Tx>;
}

impl<Tx: ExecutableTransaction> TransactionBuilderExt<Tx> for TransactionBuilder<Tx>
where
    Self: Finalizable<Tx>,
    Checked<Tx>: CheckPredicates,
{
    fn finalize_checked(&self, height: BlockHeight) -> Checked<Tx> {
        self.finalize()
            .into_checked(height, self.get_params())
            .expect("failed to check tx")
    }

    fn finalize_checked_with_storage(
        &self,
        height: BlockHeight,
        storage: &impl PredicateStorageRequirements,
    ) -> Checked<Tx> {
        let check_params = self.get_params().into();
        self.finalize()
            .into_checked_reusable_memory(
                height,
                self.get_params(),
                &check_params,
                MemoryInstance::new(),
                storage,
            )
            .expect("failed to check tx")
    }

    fn finalize_checked_basic(&self, height: BlockHeight) -> Checked<Tx> {
        self.finalize()
            .into_checked_basic(height, self.get_params())
            .expect("failed to check tx")
    }
}

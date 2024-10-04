//! Extension trait for [`fuel_tx::TransactionBuilder`]

use super::{
    Checked,
    IntoChecked,
};
use crate::{
    checked_transaction::CheckPredicates,
    prelude::*,
    storage::BlobData,
};
use fuel_storage::{
    StorageRead,
    StorageSize,
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
    fn finalize_checked(
        &self,
        height: BlockHeight,
        storage: impl StorageSize<BlobData> + StorageRead<BlobData> + Clone,
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
    fn finalize_checked(
        &self,
        height: BlockHeight,
        storage: impl StorageSize<BlobData> + StorageRead<BlobData> + Clone,
    ) -> Checked<Tx> {
        self.finalize()
            .into_checked(height, self.get_params(), storage)
            .expect("failed to check tx")
    }

    fn finalize_checked_basic(&self, height: BlockHeight) -> Checked<Tx> {
        self.finalize()
            .into_checked_basic(height, self.get_params())
            .expect("failed to check tx")
    }
}

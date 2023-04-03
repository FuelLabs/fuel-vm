// //! Extension trait for [`fuel_tx::TransactionBuilder`]
//
// use super::{Checked, IntoChecked};
// use crate::checked_transaction::CheckPredicates;
// use crate::prelude::*;
// use fuel_types::BlockHeight;
// use crate::estimated_transaction::{Estimated, IntoEstimated};
//
// /// Extension trait for [`fuel_tx::TransactionBuilder`] adding finalization methods
// pub trait TransactionBuilderExt<Tx>
// where
//     Tx: IntoEstimated,
// {
//     /// Finalize the builder into a [`Checked<Tx>`] of the correct type
//     fn finalize_checked(&mut self, height: BlockHeight, gas_costs: &GasCosts) -> Estimated<Tx>;
//
//     /// Finalize the builder into a [`Checked<Tx>`] of the correct type, with basic checks only
//     fn finalize_checked_basic(&mut self, height: BlockHeight) -> Estimated<Tx>;
// }
//
// impl<Tx: ExecutableTransaction> TransactionBuilderExt<Tx> for TransactionBuilder<Tx>
// where
//     Self: Finalizable<Tx>,
//     Estimated<Tx>: CheckPredicates,
// {
//     fn finalize_estimated(&mut self, height: BlockHeight, gas_costs: &GasCosts) -> Estimated<Tx> {
//         self.finalize()
//             .into_estimated(height, self.get_params(), gas_costs)
//             .expect("failed to check tx")
//     }
//
//     fn finalize_estimated_basic(&mut self, height: BlockHeight) -> Estimated<Tx> {
//         self.finalize()
//             .into_estimated_basic(height, self.get_params())
//             .expect("failed to check tx")
//     }
// }

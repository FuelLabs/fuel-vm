//! Implementation for different transaction types, groupd in submodules.

pub use self::create::CheckedMetadata as CreateCheckedMetadata;
pub use self::script::CheckedMetadata as ScriptCheckedMetadata;
use fuel_types::{AssetId, Word};
use std::collections::BTreeMap;

/// The spendable unrestricted initial assets.
/// More information about it in the specification:
/// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_validity.md#sufficient-balance
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct NonRetryableFreeBalances(pub(crate) BTreeMap<AssetId, Word>);

impl From<NonRetryableFreeBalances> for BTreeMap<AssetId, Word> {
    fn from(value: NonRetryableFreeBalances) -> Self {
        value.0
    }
}

impl core::ops::Deref for NonRetryableFreeBalances {
    type Target = BTreeMap<AssetId, Word>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The spendable only during execution [`AssetId::BASE`] asset.
/// More information about it in the specification:
/// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_validity.md#sufficient-balance
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RetryableAmount(pub(crate) Word);

impl From<RetryableAmount> for Word {
    fn from(value: RetryableAmount) -> Self {
        value.0
    }
}

impl core::ops::Deref for RetryableAmount {
    type Target = Word;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// For [`fuel_tx::Create`]
pub mod create {
    use super::super::{
        balances::{initial_free_balances, AvailableBalances},
        Checked, IntoChecked,
    };
    use crate::checked_transaction::{EstimatePredicates, NonRetryableFreeBalances, RetryableAmount};
    use fuel_tx::{Cacheable, CheckError, ConsensusParameters, Create, FormatValidityChecks, TransactionFee};
    use fuel_types::{BlockHeight, Word};
    use crate::error::PredicateVerificationFailed;
    use crate::gas::GasCosts;
    use crate::interpreter::{InitialBalances, Interpreter};
    use crate::storage::PredicateStorage;

    /// Metdata produced by checking [`fuel_tx::Create`].
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// See [`NonRetryableFreeBalances`].
        pub free_balances: NonRetryableFreeBalances,
        /// The block height this tx was verified with
        pub block_height: BlockHeight,
        /// The fees and gas usage
        pub fee: TransactionFee,
        /// If predicates have been checked, this is how much gas checking them used.
        /// This must be zero if the predicates have not been checked yet.
        pub gas_used_by_predicates: Word,
    }

    impl IntoChecked for Create {
        type CheckedMetadata = CheckedMetadata;

        fn into_checked_basic(
            mut self,
            block_height: BlockHeight,
            params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            self.precompute(params);
            self.check_without_signatures(block_height, params)?;

            // validate fees and compute free balances
            let AvailableBalances {
                non_retryable_balances,
                retryable_balance,
                fee,
            } = initial_free_balances(&self, params)?;
            assert_eq!(
                retryable_balance, 0,
                "The `check_without_signatures` should return `TransactionCreateMessageData` above"
            );

            let metadata = CheckedMetadata {
                free_balances: NonRetryableFreeBalances(non_retryable_balances),
                block_height,
                fee,
                gas_used_by_predicates: 0,
            };

            Ok(Checked::basic(self, metadata))
        }
    }

    // impl EstimatePredicates for Create {
    //     fn estimate_predicates(mut self, params: &ConsensusParameters, gas_costs: &GasCosts) -> Result<bool, PredicateVerificationFailed> {
    //         // validate fees and compute free balances
    //         let AvailableBalances {
    //             non_retryable_balances,
    //             retryable_balance,
    //             fee,
    //         } = initial_free_balances(&self, params).unwrap();
    //
    //         let balances: InitialBalances = InitialBalances {
    //             non_retryable: NonRetryableFreeBalances(non_retryable_balances),
    //             retryable: Some(RetryableAmount(retryable_balance)),
    //         };
    //
    //         Interpreter::<PredicateStorage>::estimate_predicates(&mut self, balances, *params, gas_costs.clone())
    //
    //     }
    // }
}

/// For [`fuel_tx::Mint`]
pub mod mint {
    use super::super::{Checked, IntoChecked};
    use fuel_tx::{Cacheable, CheckError, ConsensusParameters, FormatValidityChecks, Mint};
    use fuel_types::BlockHeight;
    use crate::checked_transaction::EstimatePredicates;
    use crate::error::PredicateVerificationFailed;
    use crate::gas::GasCosts;

    impl IntoChecked for Mint {
        type CheckedMetadata = ();

        fn into_checked_basic(
            mut self,
            block_height: BlockHeight,
            params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            self.precompute(params);
            self.check_without_signatures(block_height, params)?;

            Ok(Checked::basic(self, ()))
        }
    }

    // impl EstimatePredicates for Mint {
    //     fn estimate_predicates(mut self, _params: &ConsensusParameters, _gas_costs: &GasCosts) -> Result<bool, PredicateVerificationFailed> {
    //         Ok(true)
    //     }
    // }
}

/// For [`fuel_tx::Script`]
pub mod script {
    use super::super::{
        balances::{initial_free_balances, AvailableBalances},
        Checked, IntoChecked, ExecutableTransaction
    };
    use crate::checked_transaction::{EstimatePredicates, NonRetryableFreeBalances, RetryableAmount};
    use fuel_tx::{Cacheable, CheckError, ConsensusParameters, FormatValidityChecks, Script, TransactionFee};
    use fuel_types::{BlockHeight, Word};
    use crate::error::PredicateVerificationFailed;
    use crate::gas::GasCosts;
    use crate::interpreter::{InitialBalances, Interpreter};
    use crate::prelude::PredicateStorage;

    /// Metdata produced by checking [`fuel_tx::Script`].
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// See [`NonRetryableFreeBalances`].
        pub non_retryable_balances: NonRetryableFreeBalances,
        /// See [`RetryableAmount`].
        pub retryable_balance: RetryableAmount,
        /// The block height this tx was verified with
        pub block_height: BlockHeight,
        /// The fees and gas usage
        pub fee: TransactionFee,
        /// If predicates have been checked, this is how much gas checking them used.
        /// This must be zero if the predicates have not been checked yet.
        pub gas_used_by_predicates: Word,
    }

    impl IntoChecked for Script {
        type CheckedMetadata = CheckedMetadata;

        fn into_checked_basic(
            mut self,
            block_height: BlockHeight,
            params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            self.precompute(params);
            self.check_without_signatures(block_height, params)?;

            // validate fees and compute free balances
            let AvailableBalances {
                non_retryable_balances,
                retryable_balance,
                fee,
            } = initial_free_balances(&self, params)?;

            let metadata = CheckedMetadata {
                non_retryable_balances: NonRetryableFreeBalances(non_retryable_balances),
                retryable_balance: RetryableAmount(retryable_balance),
                block_height,
                fee,
                gas_used_by_predicates: 0,
            };

            Ok(Checked::basic(self, metadata))
        }
    }

    // impl EstimatePredicates for Script {
    //     fn estimate_predicates(mut self, params: &ConsensusParameters, gas_costs: &GasCosts) -> Result<bool, PredicateVerificationFailed> {
    //         // validate fees and compute free balances
    //         let AvailableBalances {
    //             non_retryable_balances,
    //             retryable_balance,
    //             fee,
    //         } = initial_free_balances(&self, params).unwrap();
    //
    //         let balances: InitialBalances = InitialBalances {
    //             non_retryable: NonRetryableFreeBalances(non_retryable_balances),
    //             retryable: Some(RetryableAmount(retryable_balance)),
    //         };
    //
    //         Interpreter::<PredicateStorage>::estimate_predicates(&mut self, balances, *params, gas_costs.clone())
    //     }
    // }
}

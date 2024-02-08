//! Implementation for different transaction types, groupd in submodules.

pub use self::{
    create::CheckedMetadata as CreateCheckedMetadata,
    script::CheckedMetadata as ScriptCheckedMetadata,
};
use alloc::collections::BTreeMap;
use fuel_types::{
    AssetId,
    Word,
};

/// The spendable unrestricted initial assets.
/// More information about it in the specification:
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx-validity.md#sufficient-balance>
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
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx-validity.md#sufficient-balance>
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RetryableAmount {
    pub(crate) amount: Word,
    pub(crate) base_asset_id: AssetId,
}

impl From<RetryableAmount> for Word {
    fn from(value: RetryableAmount) -> Self {
        value.amount
    }
}

impl core::ops::Deref for RetryableAmount {
    type Target = Word;

    fn deref(&self) -> &Self::Target {
        &self.amount
    }
}

/// For [`fuel_tx::Create`]
pub mod create {
    use super::super::{
        balances::{
            initial_free_balances,
            AvailableBalances,
        },
        Checked,
        IntoChecked,
    };
    use crate::checked_transaction::{
        CheckError,
        NonRetryableFreeBalances,
    };
    use fuel_tx::{
        Cacheable,
        ConsensusParameters,
        Create,
        FormatValidityChecks,
        TransactionFee,
    };
    use fuel_types::BlockHeight;

    /// Metdata produced by checking [`fuel_tx::Create`].
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// See [`NonRetryableFreeBalances`].
        pub free_balances: NonRetryableFreeBalances,
        /// The block height this tx was verified with
        pub block_height: BlockHeight,
        /// The fees and gas usage
        pub fee: TransactionFee,
    }

    impl IntoChecked for Create {
        type Metadata = CheckedMetadata;

        fn into_checked_basic(
            mut self,
            block_height: BlockHeight,
            consensus_params: &ConsensusParameters,
            gas_price: u64,
        ) -> Result<Checked<Self>, CheckError> {
            let chain_id = consensus_params.chain_id();
            self.precompute(&chain_id)?;
            self.check_without_signatures(block_height, consensus_params, gas_price)?;

            // validate fees and compute free balances
            let AvailableBalances {
                non_retryable_balances,
                retryable_balance,
                fee,
            } = initial_free_balances(
                &self,
                consensus_params.gas_costs(),
                consensus_params.fee_params(),
                consensus_params.base_asset_id(),
            )?;
            assert_eq!(
                retryable_balance, 0,
                "The `check_without_signatures` should return `TransactionCreateMessageData` above"
            );

            let metadata = CheckedMetadata {
                free_balances: NonRetryableFreeBalances(non_retryable_balances),
                block_height,
                fee,
            };

            Ok(Checked::basic(self, metadata))
        }
    }
}

/// For [`fuel_tx::Mint`]
pub mod mint {
    use super::super::{
        Checked,
        IntoChecked,
    };
    use crate::checked_transaction::CheckError;
    use fuel_tx::{
        Cacheable,
        ConsensusParameters,
        FormatValidityChecks,
        Mint,
    };
    use fuel_types::BlockHeight;

    impl IntoChecked for Mint {
        type Metadata = ();

        fn into_checked_basic(
            mut self,
            block_height: BlockHeight,
            consensus_params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            let chain_id = consensus_params.chain_id();
            self.precompute(&chain_id)?;
            self.check_without_signatures(block_height, consensus_params)?;

            Ok(Checked::basic(self, ()))
        }
    }
}

/// For [`fuel_tx::Script`]
pub mod script {
    use super::super::{
        balances::{
            initial_free_balances,
            AvailableBalances,
        },
        Checked,
        IntoChecked,
    };
    use crate::checked_transaction::{
        CheckError,
        NonRetryableFreeBalances,
        RetryableAmount,
    };
    use fuel_tx::{
        Cacheable,
        ConsensusParameters,
        FormatValidityChecks,
        Script,
        TransactionFee,
    };
    use fuel_types::BlockHeight;

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
    }

    impl IntoChecked for Script {
        type Metadata = CheckedMetadata;

        fn into_checked_basic(
            mut self,
            block_height: BlockHeight,
            consensus_params: &ConsensusParameters,
            gas_price: u64,
        ) -> Result<Checked<Self>, CheckError> {
            let chain_id = consensus_params.chain_id();
            self.precompute(&chain_id)?;
            self.check_without_signatures(block_height, consensus_params)?;

            // validate fees and compute free balances
            let AvailableBalances {
                non_retryable_balances,
                retryable_balance,
                fee,
            } = initial_free_balances(
                &self,
                consensus_params.gas_costs(),
                consensus_params.fee_params(),
                consensus_params.base_asset_id(),
                gas_price,
            )?;

            let metadata = CheckedMetadata {
                non_retryable_balances: NonRetryableFreeBalances(non_retryable_balances),
                retryable_balance: RetryableAmount {
                    amount: retryable_balance,
                    base_asset_id: consensus_params.base_asset_id,
                },
                block_height,
                fee,
            };

            Ok(Checked::basic(self, metadata))
        }
    }
}

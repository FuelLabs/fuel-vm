//! Implementation for different transaction types, groupd in submodules.

pub use self::{
    blob::CheckedMetadata as BlobCheckedMetadata,
    create::CheckedMetadata as CreateCheckedMetadata,
    script::CheckedMetadata as ScriptCheckedMetadata,
    upgrade::CheckedMetadata as UpgradeCheckedMetadata,
    upload::CheckedMetadata as UploadCheckedMetadata,
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
        Checked,
        IntoChecked,
        balances::{
            AvailableBalances,
            initial_free_balances,
        },
    };
    use crate::checked_transaction::{
        CheckError,
        NonRetryableFreeBalances,
    };
    use fuel_tx::{
        Cacheable,
        Chargeable,
        ConsensusParameters,
        Create,
        FormatValidityChecks,
    };
    use fuel_types::{
        AssetId,
        BlockHeight,
    };

    /// Metadata produced by checking [`fuel_tx::Create`].
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// The base asset id.
        pub base_asset_id: AssetId,
        /// See [`NonRetryableFreeBalances`].
        pub free_balances: NonRetryableFreeBalances,
        /// The block height this tx was verified with
        pub block_height: BlockHeight,
        /// The minimum gas required for this transaction.
        pub min_gas: u64,
        /// The maximum gas required for this transaction.
        pub max_gas: u64,
    }

    impl IntoChecked for Create {
        type Metadata = CheckedMetadata;

        fn into_checked_basic(
            mut self,
            block_height: BlockHeight,
            consensus_params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            let chain_id = consensus_params.chain_id();
            self.precompute(&chain_id)?;
            self.check_without_signatures(block_height, consensus_params)?;

            // validate fees and compute free balances
            let AvailableBalances {
                non_retryable_balances,
                retryable_balance,
            } = initial_free_balances(&self, consensus_params.base_asset_id())?;
            debug_assert_eq!(
                retryable_balance, 0,
                "The `check_without_signatures` should return `TransactionInputContainsMessageData` above"
            );

            let metadata = CheckedMetadata {
                base_asset_id: *consensus_params.base_asset_id(),
                free_balances: NonRetryableFreeBalances(non_retryable_balances),
                block_height,
                min_gas: self
                    .min_gas(consensus_params.gas_costs(), consensus_params.fee_params()),
                max_gas: self
                    .max_gas(consensus_params.gas_costs(), consensus_params.fee_params()),
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
        Checked,
        IntoChecked,
        balances::{
            AvailableBalances,
            initial_free_balances,
        },
    };
    use crate::checked_transaction::{
        CheckError,
        NonRetryableFreeBalances,
        RetryableAmount,
    };
    #[cfg(feature = "chargeable-tx-v2")]
    use fuel_tx::ScriptV2;
    use fuel_tx::{
        Cacheable,
        Chargeable,
        ConsensusParameters,
        FormatValidityChecks,
        Script,
    };
    use fuel_types::{
        AssetId,
        BlockHeight,
    };

    /// Metadata produced by checking [`fuel_tx::Script`].
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// The base asset id.
        pub base_asset_id: AssetId,
        /// See [`NonRetryableFreeBalances`].
        pub non_retryable_balances: NonRetryableFreeBalances,
        /// See [`RetryableAmount`].
        pub retryable_balance: RetryableAmount,
        /// The block height this tx was verified with
        pub block_height: BlockHeight,
        /// The minimum gas required for this transaction.
        pub min_gas: u64,
        /// The maximum gas required for this transaction.
        pub max_gas: u64,
    }

    impl IntoChecked for Script {
        type Metadata = CheckedMetadata;

        fn into_checked_basic(
            mut self,
            block_height: BlockHeight,
            consensus_params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            let chain_id = consensus_params.chain_id();
            self.precompute(&chain_id)?;
            self.check_without_signatures(block_height, consensus_params)?;

            // validate fees and compute free balances
            let AvailableBalances {
                non_retryable_balances,
                retryable_balance,
            } = initial_free_balances(&self, consensus_params.base_asset_id())?;

            let metadata = CheckedMetadata {
                base_asset_id: *consensus_params.base_asset_id(),
                non_retryable_balances: NonRetryableFreeBalances(non_retryable_balances),
                retryable_balance: RetryableAmount {
                    amount: retryable_balance,
                    base_asset_id: *consensus_params.base_asset_id(),
                },
                block_height,
                min_gas: self
                    .min_gas(consensus_params.gas_costs(), consensus_params.fee_params()),
                max_gas: self
                    .max_gas(consensus_params.gas_costs(), consensus_params.fee_params()),
            };

            Ok(Checked::basic(self, metadata))
        }
    }

    #[cfg(feature = "chargeable-tx-v2")]
    impl IntoChecked for ScriptV2 {
        type Metadata = CheckedMetadata;

        fn into_checked_basic(
            self,
            _block_height: BlockHeight,
            _consensus_params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            todo!()
        }
    }
}

/// For [`fuel_tx::Upgrade`]
pub mod upgrade {
    use super::super::{
        Checked,
        IntoChecked,
        balances::{
            AvailableBalances,
            initial_free_balances,
        },
    };
    use crate::checked_transaction::{
        CheckError,
        NonRetryableFreeBalances,
    };
    use fuel_tx::{
        Cacheable,
        Chargeable,
        ConsensusParameters,
        FormatValidityChecks,
        Upgrade,
    };
    use fuel_types::{
        AssetId,
        BlockHeight,
    };

    /// Metadata produced by checking [`fuel_tx::Upgrade`].
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// The base asset id.
        pub base_asset_id: AssetId,
        /// See [`NonRetryableFreeBalances`].
        pub free_balances: NonRetryableFreeBalances,
        /// The block height this tx was verified with
        pub block_height: BlockHeight,
        /// The minimum gas required for this transaction.
        pub min_gas: u64,
        /// The maximum gas required for this transaction.
        pub max_gas: u64,
    }

    impl IntoChecked for Upgrade {
        type Metadata = CheckedMetadata;

        fn into_checked_basic(
            mut self,
            block_height: BlockHeight,
            consensus_params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            let chain_id = consensus_params.chain_id();
            self.precompute(&chain_id)?;
            self.check_without_signatures(block_height, consensus_params)?;

            // validate fees and compute free balances
            let AvailableBalances {
                non_retryable_balances,
                retryable_balance,
            } = initial_free_balances(&self, consensus_params.base_asset_id())?;
            debug_assert_eq!(
                retryable_balance, 0,
                "The `check_without_signatures` should return `TransactionInputContainsMessageData` above"
            );

            let metadata = CheckedMetadata {
                base_asset_id: *consensus_params.base_asset_id(),
                free_balances: NonRetryableFreeBalances(non_retryable_balances),
                block_height,
                min_gas: self
                    .min_gas(consensus_params.gas_costs(), consensus_params.fee_params()),
                max_gas: self
                    .max_gas(consensus_params.gas_costs(), consensus_params.fee_params()),
            };

            Ok(Checked::basic(self, metadata))
        }
    }
}

/// For [`fuel_tx::Upload`]
pub mod upload {
    use super::super::{
        Checked,
        IntoChecked,
        balances::{
            AvailableBalances,
            initial_free_balances,
        },
    };
    use crate::checked_transaction::{
        CheckError,
        NonRetryableFreeBalances,
    };
    use fuel_tx::{
        Cacheable,
        Chargeable,
        ConsensusParameters,
        FormatValidityChecks,
        Upload,
    };
    use fuel_types::{
        AssetId,
        BlockHeight,
    };

    /// Metadata produced by checking [`fuel_tx::Upload`].
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// The base asset id.
        pub base_asset_id: AssetId,
        /// See [`NonRetryableFreeBalances`].
        pub free_balances: NonRetryableFreeBalances,
        /// The block height this tx was verified with
        pub block_height: BlockHeight,
        /// The minimum gas required for this transaction.
        pub min_gas: u64,
        /// The maximum gas required for this transaction.
        pub max_gas: u64,
    }

    impl IntoChecked for Upload {
        type Metadata = CheckedMetadata;

        fn into_checked_basic(
            mut self,
            block_height: BlockHeight,
            consensus_params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            let chain_id = consensus_params.chain_id();
            self.precompute(&chain_id)?;
            self.check_without_signatures(block_height, consensus_params)?;

            // validate fees and compute free balances
            let AvailableBalances {
                non_retryable_balances,
                retryable_balance,
            } = initial_free_balances(&self, consensus_params.base_asset_id())?;
            debug_assert_eq!(
                retryable_balance, 0,
                "The `check_without_signatures` should return `TransactionInputContainsMessageData` above"
            );

            let metadata = CheckedMetadata {
                base_asset_id: *consensus_params.base_asset_id(),
                free_balances: NonRetryableFreeBalances(non_retryable_balances),
                block_height,
                min_gas: self
                    .min_gas(consensus_params.gas_costs(), consensus_params.fee_params()),
                max_gas: self
                    .max_gas(consensus_params.gas_costs(), consensus_params.fee_params()),
            };

            Ok(Checked::basic(self, metadata))
        }
    }
}

/// For [`fuel_tx::Blob`]
pub mod blob {
    use super::super::{
        Checked,
        IntoChecked,
        balances::{
            AvailableBalances,
            initial_free_balances,
        },
    };
    use crate::checked_transaction::{
        CheckError,
        NonRetryableFreeBalances,
    };
    use fuel_tx::{
        AssetId,
        Blob,
        Cacheable,
        Chargeable,
        ConsensusParameters,
        FormatValidityChecks,
    };
    use fuel_types::BlockHeight;

    /// Metadata produced by checking [`fuel_tx::Blob`].
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// The base asset id.
        pub base_asset_id: AssetId,
        /// See [`NonRetryableFreeBalances`].
        pub free_balances: NonRetryableFreeBalances,
        /// The block height this tx was verified with
        pub block_height: BlockHeight,
        /// The minimum gas required for this transaction.
        pub min_gas: u64,
        /// The maximum gas required for this transaction.
        pub max_gas: u64,
    }

    impl IntoChecked for Blob {
        type Metadata = CheckedMetadata;

        fn into_checked_basic(
            mut self,
            block_height: BlockHeight,
            consensus_params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            let chain_id = consensus_params.chain_id();
            self.precompute(&chain_id)?;
            self.check_without_signatures(block_height, consensus_params)?;

            // validate fees and compute free balances
            let AvailableBalances {
                non_retryable_balances,
                retryable_balance,
            } = initial_free_balances(&self, consensus_params.base_asset_id())?;
            debug_assert_eq!(
                retryable_balance, 0,
                "The `check_without_signatures` should return `TransactionInputContainsMessageData` above"
            );

            let metadata = CheckedMetadata {
                base_asset_id: *consensus_params.base_asset_id(),
                free_balances: NonRetryableFreeBalances(non_retryable_balances),
                block_height,
                min_gas: self
                    .min_gas(consensus_params.gas_costs(), consensus_params.fee_params()),
                max_gas: self
                    .max_gas(consensus_params.gas_costs(), consensus_params.fee_params()),
            };

            Ok(Checked::basic(self, metadata))
        }
    }
}

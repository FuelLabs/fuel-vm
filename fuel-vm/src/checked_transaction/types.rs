//! Implementation for different transaction types, groupd in submodules.

pub use self::create::CheckedMetadata as CreateCheckedMetadata;
pub use self::script::CheckedMetadata as ScriptCheckedMetadata;
use fuel_types::{AssetId, Word};
use std::collections::BTreeMap;

/// The spendable unrestricted initial assets.
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct SumInputs(pub(crate) BTreeMap<AssetId, Word>);

impl From<SumInputs> for BTreeMap<AssetId, Word> {
    fn from(value: SumInputs) -> Self {
        value.0
    }
}

impl core::ops::Deref for SumInputs {
    type Target = BTreeMap<AssetId, Word>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The spendable only during execution [`AssetId::BASE`] asset.
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct SumDataMessages(pub(crate) Word);

impl From<SumDataMessages> for Word {
    fn from(value: SumDataMessages) -> Self {
        value.0
    }
}

impl core::ops::Deref for SumDataMessages {
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
    use crate::checked_transaction::SumInputs;
    use fuel_tx::{Cacheable, CheckError, ConsensusParameters, Create, FormatValidityChecks, TransactionFee};
    use fuel_types::Word;

    /// Metdata produced by checking [`fuel_tx::Create`].
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// See [`SumInputs`].
        pub sum_inputs: SumInputs,
        /// The block height this tx was verified with
        pub block_height: Word,
        /// The fees and gas usage
        pub fee: TransactionFee,
        /// If predicates have been checked, this is how much gas checking them used.
        /// This must be zero if the predicates have not been checked yet.
        pub gas_used_by_predicates: Word,
    }

    impl IntoChecked for Create {
        type Metadata = CheckedMetadata;

        fn into_checked_basic(
            mut self,
            block_height: Word,
            params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            self.precompute();
            self.check_without_signatures(block_height, params)?;

            // validate fees and compute free balances
            let AvailableBalances {
                sum_inputs,
                sum_data_messages,
                fee,
            } = initial_free_balances(&self, params)?;
            assert_eq!(
                sum_data_messages, 0,
                "The `check_without_signatures` should return `TransactionCreateMetadata` above"
            );

            let metadata = CheckedMetadata {
                sum_inputs: SumInputs(sum_inputs),
                block_height,
                fee,
                gas_used_by_predicates: 0,
            };

            Ok(Checked::basic(self, metadata))
        }
    }
}

/// For [`fuel_tx::Mint`]
pub mod mint {
    use super::super::{Checked, IntoChecked};
    use fuel_tx::{Cacheable, CheckError, ConsensusParameters, FormatValidityChecks, Mint};
    use fuel_types::Word;

    impl IntoChecked for Mint {
        type Metadata = ();

        fn into_checked_basic(
            mut self,
            block_height: Word,
            params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            self.precompute();
            self.check_without_signatures(block_height, params)?;

            Ok(Checked::basic(self, ()))
        }
    }
}

/// For [`fuel_tx::Script`]
pub mod script {
    use super::super::{
        balances::{initial_free_balances, AvailableBalances},
        Checked, IntoChecked,
    };
    use crate::checked_transaction::{SumDataMessages, SumInputs};
    use fuel_tx::{Cacheable, CheckError, ConsensusParameters, FormatValidityChecks, Script, TransactionFee};
    use fuel_types::Word;

    /// Metdata produced by checking [`fuel_tx::Script`].
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// See [`SumInputs`].
        pub sum_inputs: SumInputs,
        /// See [`SumDataMessages`].
        pub sum_data_messages: SumDataMessages,
        /// The block height this tx was verified with
        pub block_height: Word,
        /// The fees and gas usage
        pub fee: TransactionFee,
        /// If predicates have been checked, this is how much gas checking them used.
        /// This must be zero if the predicates have not been checked yet.
        pub gas_used_by_predicates: Word,
    }

    impl IntoChecked for Script {
        type Metadata = CheckedMetadata;

        fn into_checked_basic(
            mut self,
            block_height: Word,
            params: &ConsensusParameters,
        ) -> Result<Checked<Self>, CheckError> {
            self.precompute();
            self.check_without_signatures(block_height, params)?;

            // validate fees and compute free balances
            let AvailableBalances {
                sum_inputs,
                sum_data_messages,
                fee,
            } = initial_free_balances(&self, params)?;

            let metadata = CheckedMetadata {
                sum_inputs: SumInputs(sum_inputs),
                sum_data_messages: SumDataMessages(sum_data_messages),
                block_height,
                fee,
                gas_used_by_predicates: 0,
            };

            Ok(Checked::basic(self, metadata))
        }
    }
}

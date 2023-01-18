#![allow(missing_docs)]

pub use self::create::CheckedMetadata as CreateCheckedMetadata;
pub use self::script::CheckedMetadata as ScriptCheckedMetadata;

pub mod create {
    use super::super::{initial_free_balances, AvailableBalances, Checked, IntoChecked};
    use fuel_tx::{Cacheable, CheckError, ConsensusParameters, Create, FormatValidityChecks, TransactionFee};
    use fuel_types::{AssetId, Word};
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// The mapping of initial free balances
        pub initial_free_balances: BTreeMap<AssetId, Word>,
        /// The block height this tx was verified with
        pub block_height: Word,
        /// The fees and gas usage
        pub fee: TransactionFee,
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
                initial_free_balances,
                fee,
            } = initial_free_balances(&self, params)?;

            let metadata = CheckedMetadata {
                initial_free_balances,
                block_height,
                fee,
            };

            Ok(Checked::basic(self, metadata))
        }
    }
}

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

pub mod script {
    use super::super::{initial_free_balances, AvailableBalances, Checked, IntoChecked};
    use fuel_tx::{Cacheable, CheckError, ConsensusParameters, FormatValidityChecks, Script, TransactionFee};
    use fuel_types::{AssetId, Word};
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub struct CheckedMetadata {
        /// The mapping of initial free balances
        pub initial_free_balances: BTreeMap<AssetId, Word>,
        /// The block height this tx was verified with
        pub block_height: Word,
        /// The fees and gas usage
        pub fee: TransactionFee,
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
                initial_free_balances,
                fee,
            } = initial_free_balances(&self, params)?;

            let metadata = CheckedMetadata {
                initial_free_balances,
                block_height,
                fee,
            };

            Ok(Checked::basic(self, metadata))
        }
    }
}

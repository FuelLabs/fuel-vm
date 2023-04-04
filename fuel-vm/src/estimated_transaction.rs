//! An estimated transaction is type-wrapper for transactions which have had the gas for each of
//! their predicates estimated.

#![allow(non_upper_case_globals)]

use fuel_tx::{CheckError, ConsensusParameters, Create, Mint, Script, Transaction};
use fuel_types::BlockHeight;

use core::borrow::Borrow;

mod balances;
pub mod builder;
pub mod types;

pub use types::*;

use crate::{gas::GasCosts, interpreter::EstimatedMetadata as EstimatedMetadataAccessTrait, prelude::*};

bitflags::bitflags! {
    /// Possible types of transaction checks.
    pub struct Checks: u32 {
        /// Basic checks defined in the specification for each transaction:
        /// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_format/transaction.md#transaction
        const Basic         = 0b00000001;
        /// Check that signature in the transactions are valid.
        const Signatures    = 0b00000010;
        /// Check that estimation for predicates was successful.
        const Estimates    = 0b00000100;
        /// All possible checks.
        const All           = Self::Basic.bits
                            | Self::Signatures.bits
                            | Self::Estimates.bits;
    }
}

impl core::fmt::Display for Checks {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:032b}", self.bits)
    }
}

/// The type describes that the inner transaction was already estimated.
///
/// All fields are private, and there is no constructor, so it is impossible to create the instance
/// of `EEstimated` outside the `fuel-tx` crate.
///
/// The inner data is immutable to prevent modification to invalidate the checking.
///
/// If you need to modify an inner state, you need to get inner values
/// (via the `Into<(Tx, Tx ::Metadata)>` trait), modify them and check again.
///
/// # Dev note: Avoid serde serialization of this type.
///
/// Since estimated tx would need to be re-validated on deserialization anyways,
/// it's cleaner to redo the tx check.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Estimated<Tx: IntoEstimated> {
    transaction: Tx,
    metadata: Tx::EstimatedMetadata,
    checks_bitmask: Checks,
}

impl<Tx: IntoEstimated> Estimated<Tx> {
    fn new(transaction: Tx, metadata: Tx::EstimatedMetadata, checks_bitmask: Checks) -> Self {
        Estimated {
            transaction,
            metadata,
            checks_bitmask,
        }
    }

    pub(crate) fn basic(transaction: Tx, metadata: Tx::EstimatedMetadata) -> Self {
        Estimated::new(transaction, metadata, Checks::Basic)
    }

    /// Returns reference on inner transaction.
    pub fn transaction(&self) -> &Tx {
        &self.transaction
    }

    /// Returns reference on inner transaction.
    pub fn transaction_mut(&mut self) -> &mut Tx {
        &mut self.transaction
    }

    /// Returns the metadata generated during the check for transaction.
    pub fn metadata(&self) -> &Tx::EstimatedMetadata {
        &self.metadata
    }

    /// Returns the bitmask of all passed checks.
    pub fn checks(&self) -> &Checks {
        &self.checks_bitmask
    }

    /// Performs check of signatures, if not yet done.
    pub fn check_signatures(mut self, parameters: &ConsensusParameters) -> Result<Self, CheckError> {
        if !self.checks_bitmask.contains(Checks::Signatures) {
            self.transaction.check_signatures(parameters)?;
            self.checks_bitmask.insert(Checks::Signatures);
        }
        Ok(self)
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx: IntoEstimated + Default> Default for Estimated<Tx>
where
    Estimated<Tx>: EstimatePredicates,
{
    fn default() -> Self {
        Tx::default()
            .into_estimated(Default::default(), &Default::default(), &Default::default())
            .expect("default tx should produce a valid fully estimated transaction")
    }
}

impl<Tx: IntoEstimated> From<Estimated<Tx>> for (Tx, Tx::EstimatedMetadata) {
    fn from(estimated: Estimated<Tx>) -> Self {
        let Estimated {
            transaction, metadata, ..
        } = estimated;

        (transaction, metadata)
    }
}

impl<Tx: IntoEstimated> AsRef<Tx> for Estimated<Tx> {
    fn as_ref(&self) -> &Tx {
        &self.transaction
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx: IntoEstimated> AsMut<Tx> for Estimated<Tx> {
    fn as_mut(&mut self) -> &mut Tx {
        &mut self.transaction
    }
}

impl<Tx: IntoEstimated> Borrow<Tx> for Estimated<Tx> {
    fn borrow(&self) -> &Tx {
        self.transaction()
    }
}

/// Performs estimation for a transaction
pub trait IntoEstimated: FormatValidityChecks + Sized {
    /// Metadata produced during the check.
    type EstimatedMetadata: Sized + Clone;

    /// Returns transaction that passed all `Checks`.
    fn into_estimated(
        self,
        block_height: BlockHeight,
        params: &ConsensusParameters,
        gas_costs: &GasCosts,
    ) -> Result<Estimated<Self>, CheckError>
    where
        Estimated<Self>: EstimatePredicates,
    {
        self.into_estimated_basic(block_height, params)?
            .check_signatures(params)?
            .estimate_predicates(params, gas_costs)
    }

    /// Returns transaction that passed only `Checks::Basic`.
    fn into_estimated_basic(
        self,
        block_height: BlockHeight,
        params: &ConsensusParameters,
    ) -> Result<Estimated<Self>, CheckError>;
}

/// Performs predicate verification for a transaction
pub trait EstimatePredicates: Sized {
    /// Define predicate verification logic (if any)
    fn estimate_predicates(self, params: &ConsensusParameters, gas_costs: &GasCosts) -> Result<Self, CheckError>;
}

impl<Tx: ExecutableTransaction> EstimatePredicates for Estimated<Tx>
where
    Self: Clone,
    <Tx as IntoEstimated>::EstimatedMetadata: crate::interpreter::EstimatedMetadata,
{
    fn estimate_predicates(mut self, params: &ConsensusParameters, gas_costs: &GasCosts) -> Result<Self, CheckError> {
        if !self.checks_bitmask.contains(Checks::Estimates) {
            // TODO: Optimize predicate verification to work with references where it is possible.
            let estimated =
                Interpreter::<PredicateStorage>::estimate_predicates(self.clone(), *params, gas_costs.clone())?;
            self.checks_bitmask.insert(Checks::Estimates);
            self.metadata.set_gas_used_by_predicates(estimated.gas_used());
        }
        Ok(self)
    }
}

impl EstimatePredicates for Estimated<Mint> {
    fn estimate_predicates(mut self, params: &ConsensusParameters, gas_costs: &GasCosts) -> Result<Self, CheckError> {
        self.checks_bitmask.insert(Checks::Estimates);
        Ok(self)
    }
}

impl EstimatePredicates for Estimated<Transaction> {
    fn estimate_predicates(self, params: &ConsensusParameters, gas_costs: &GasCosts) -> Result<Self, CheckError> {
        let estimated_transaction: EstimatedTransaction = self.into();
        let estimated_transaction: EstimatedTransaction = match estimated_transaction {
            EstimatedTransaction::Script(tx) => EstimatePredicates::estimate_predicates(tx, params, gas_costs)?.into(),
            EstimatedTransaction::Create(tx) => EstimatePredicates::estimate_predicates(tx, params, gas_costs)?.into(),
            EstimatedTransaction::Mint(tx) => EstimatePredicates::estimate_predicates(tx, params, gas_costs)?.into(),
        };
        Ok(estimated_transaction.into())
    }
}

/// The Enum version of `Estimated<Transaction>` allows getting the inner variant without losing
/// "estimated" status.
///
/// It is possible to freely convert `Estimated<Transaction>` into `EstimatedTransaction` and vice
/// verse without the overhead.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub enum EstimatedTransaction {
    Script(Estimated<Script>),
    Create(Estimated<Create>),
    Mint(Estimated<Mint>),
}

impl From<Estimated<Transaction>> for EstimatedTransaction {
    fn from(estimated: Estimated<Transaction>) -> Self {
        let Estimated {
            transaction,
            metadata,
            checks_bitmask,
        } = estimated;

        // # Dev note: Avoid wildcard pattern to be sure that all variants are covered.
        match (transaction, metadata) {
            (Transaction::Script(transaction), EstimatedMetadata::Script(metadata)) => {
                Self::Script(Estimated::new(transaction, metadata, checks_bitmask))
            }
            (Transaction::Create(transaction), EstimatedMetadata::Create(metadata)) => {
                Self::Create(Estimated::new(transaction, metadata, checks_bitmask))
            }
            (Transaction::Mint(transaction), EstimatedMetadata::Mint(metadata)) => {
                Self::Mint(Estimated::new(transaction, metadata, checks_bitmask))
            }
            // The code should produce the `EstimatedMetadata` for the corresponding transaction
            // variant. It is done in the implementation of the `IntoEstimated` trait for
            // `Transaction`. With the current implementation, the patterns below are unreachable.
            (Transaction::Script(_), _) => unreachable!(),
            (Transaction::Create(_), _) => unreachable!(),
            (Transaction::Mint(_), _) => unreachable!(),
        }
    }
}

impl From<Estimated<Script>> for EstimatedTransaction {
    fn from(estimated: Estimated<Script>) -> Self {
        Self::Script(estimated)
    }
}

impl From<Estimated<Create>> for EstimatedTransaction {
    fn from(estimated: Estimated<Create>) -> Self {
        Self::Create(estimated)
    }
}

impl From<Estimated<Mint>> for EstimatedTransaction {
    fn from(estimated: Estimated<Mint>) -> Self {
        Self::Mint(estimated)
    }
}

impl From<EstimatedTransaction> for Estimated<Transaction> {
    fn from(estimated: EstimatedTransaction) -> Self {
        match estimated {
            EstimatedTransaction::Script(Estimated {
                transaction,
                metadata,
                checks_bitmask,
            }) => Estimated::new(transaction.into(), metadata.into(), checks_bitmask),
            EstimatedTransaction::Create(Estimated {
                transaction,
                metadata,
                checks_bitmask,
            }) => Estimated::new(transaction.into(), metadata.into(), checks_bitmask),
            EstimatedTransaction::Mint(Estimated {
                transaction,
                metadata,
                checks_bitmask,
            }) => Estimated::new(transaction.into(), metadata.into(), checks_bitmask),
        }
    }
}

/// The `IntoEstimated` metadata for `EstimatedTransaction`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub enum EstimatedMetadata {
    Script(<Script as IntoEstimated>::EstimatedMetadata),
    Create(<Create as IntoEstimated>::EstimatedMetadata),
    Mint(<Mint as IntoEstimated>::EstimatedMetadata),
}

impl From<<Script as IntoEstimated>::EstimatedMetadata> for EstimatedMetadata {
    fn from(metadata: <Script as IntoEstimated>::EstimatedMetadata) -> Self {
        Self::Script(metadata)
    }
}

impl From<<Create as IntoEstimated>::EstimatedMetadata> for EstimatedMetadata {
    fn from(metadata: <Create as IntoEstimated>::EstimatedMetadata) -> Self {
        Self::Create(metadata)
    }
}

impl From<<Mint as IntoEstimated>::EstimatedMetadata> for EstimatedMetadata {
    fn from(metadata: <Mint as IntoEstimated>::EstimatedMetadata) -> Self {
        Self::Mint(metadata)
    }
}

impl IntoEstimated for Transaction {
    type EstimatedMetadata = EstimatedMetadata;

    fn into_estimated_basic(
        self,
        block_height: BlockHeight,
        params: &ConsensusParameters,
    ) -> Result<Estimated<Self>, CheckError> {
        let (transaction, metadata) = match self {
            Transaction::Script(script) => {
                let (transaction, metadata) = script.into_estimated_basic(block_height, params)?.into();
                (transaction.into(), metadata.into())
            }
            Transaction::Create(create) => {
                let (transaction, metadata) = create.into_estimated_basic(block_height, params)?.into();
                (transaction.into(), metadata.into())
            }
            Transaction::Mint(mint) => {
                let (transaction, metadata) = mint.into_estimated_basic(block_height, params)?.into();
                (transaction.into(), metadata.into())
            }
        };

        Ok(Estimated::basic(transaction, metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fuel_asm::op;
    use fuel_crypto::SecretKey;
    use fuel_tx::{CheckError, Script, TransactionBuilder};
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn estimated_tx_has_default() {
        let height = 1;

        Estimated::<Transaction>::default()
            .transaction()
            .estimate(height, &Default::default())
            .expect("default estimated tx should be valid");
    }

    #[test]
    fn estimated_tx_accepts_valid_tx() {
        // simple smoke test that valid txs can be estimated
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_price = 10;
        let gas_limit = 1000;
        let input_amount = 1000;
        let output_amount = 10;
        let tx = valid_coin_tx(rng, gas_price, gas_limit, input_amount, output_amount);

        let estimated = tx
            .clone()
            .into_estimated(0, &ConsensusParameters::DEFAULT, &Default::default())
            .expect("Expected valid transaction");

        // verify transaction getter works
        assert_eq!(estimated.transaction(), &tx);
        // verify available balance was decreased by max fee
        assert_eq!(
            estimated.metadata().initial_free_balances[&AssetId::default()],
            input_amount - estimated.metadata().fee.total() - output_amount
        );
    }

    #[test]
    fn checked_tx_accepts_valid_signed_message_input_fees() {
        // simple test to ensure a tx that only has a message input can cover fees
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 100;
        let output_amount = 0;
        let gas_price = 100;
        let gas_limit = 1000;
        let tx = signed_message_tx(rng, gas_price, gas_limit, input_amount, output_amount);

        let checked = tx
            .into_checked(0, &ConsensusParameters::DEFAULT, &Default::default())
            .expect("Expected valid transaction");

        // verify available balance was decreased by max fee
        assert_eq!(
            checked.metadata().initial_free_balances[&AssetId::default()],
            input_amount - checked.metadata().fee.total()
        );
    }

    #[test]
    fn checked_tx_excludes_message_output_amount_from_fee() {
        // ensure message outputs aren't deducted from available balance
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 100;
        // set a large message output amount
        let output_amount = u64::MAX;
        let gas_price = 100;
        let gas_limit = 1000;
        let tx = signed_message_tx(rng, gas_price, gas_limit, input_amount, output_amount);

        let checked = tx
            .into_checked(0, &ConsensusParameters::DEFAULT, &Default::default())
            .expect("Expected valid transaction");

        // verify available balance was decreased by max fee
        assert_eq!(
            checked.metadata().initial_free_balances[&AssetId::default()],
            input_amount - checked.metadata().fee.total()
        );
    }

    // use quickcheck to fuzz any rounding or precision errors in the max fee w/ coin input
    #[quickcheck]
    fn max_fee_coin_input(
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        gas_price_factor: u64,
        seed: u64,
    ) -> TestResult {
        // verify max fee a transaction can consume based on gas limit + bytes is correct

        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard();
        }

        let rng = &mut StdRng::seed_from_u64(seed);
        let params = ConsensusParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let tx = predicate_tx(rng, gas_price, gas_limit, input_amount);

        if let Ok(valid) = is_valid_max_fee(&tx, &params) {
            TestResult::from_bool(valid)
        } else {
            TestResult::discard()
        }
    }

    // use quickcheck to fuzz any rounding or precision errors in the min fee w/ coin input
    #[quickcheck]
    fn min_fee_coin_input(
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        gas_price_factor: u64,
        seed: u64,
    ) -> TestResult {
        // verify min fee a transaction can consume based on bytes is correct

        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard();
        }
        let rng = &mut StdRng::seed_from_u64(seed);
        let params = ConsensusParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let tx = predicate_tx(rng, gas_price, gas_limit, input_amount);

        if let Ok(valid) = is_valid_max_fee(&tx, &params) {
            TestResult::from_bool(valid)
        } else {
            TestResult::discard()
        }
    }

    // use quickcheck to fuzz any rounding or precision errors in the max fee w/ message input
    #[quickcheck]
    fn max_fee_message_input(
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        gas_price_factor: u64,
        seed: u64,
    ) -> TestResult {
        // verify max fee a transaction can consume based on gas limit + bytes is correct

        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard();
        }

        let rng = &mut StdRng::seed_from_u64(seed);
        let params = ConsensusParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let tx = predicate_message_tx(rng, gas_price, gas_limit, input_amount, 0);

        if let Ok(valid) = is_valid_max_fee(&tx, &params) {
            TestResult::from_bool(valid)
        } else {
            TestResult::discard()
        }
    }

    // use quickcheck to fuzz any rounding or precision errors in the min fee w/ message input
    #[quickcheck]
    fn min_fee_message_input(
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        gas_price_factor: u64,
        seed: u64,
    ) -> TestResult {
        // verify min fee a transaction can consume based on bytes is correct

        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard();
        }
        let rng = &mut StdRng::seed_from_u64(seed);
        let params = ConsensusParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let tx = predicate_message_tx(rng, gas_price, gas_limit, input_amount, 0);

        if let Ok(valid) = is_valid_min_fee(&tx, &params) {
            TestResult::from_bool(valid)
        } else {
            TestResult::discard()
        }
    }

    #[test]
    fn checked_tx_rejects_invalid_tx() {
        // simple smoke test that invalid txs cannot be checked
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let asset = rng.gen();
        let gas_price = 1;
        let gas_limit = 100;
        let input_amount = 1_000;

        // create a tx with invalid signature
        let tx = TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_input(Input::coin_signed(
                rng.gen(),
                rng.gen(),
                input_amount,
                asset,
                rng.gen(),
                0,
                0,
            ))
            .add_input(Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()))
            .add_output(Output::contract(1, rng.gen(), rng.gen()))
            .add_output(Output::coin(rng.gen(), 10, asset))
            .add_output(Output::change(rng.gen(), 0, asset))
            .add_witness(Default::default())
            .finalize();

        let checked = tx
            .into_checked(0, &ConsensusParameters::DEFAULT, &Default::default())
            .expect_err("Expected invalid transaction");

        // assert that tx without base input assets fails
        assert_eq!(
            CheckError::InsufficientFeeAmount {
                expected: 1,
                provided: 0
            },
            checked
        );
    }

    #[test]
    fn checked_tx_fails_when_provided_fees_dont_cover_byte_costs() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let input_amount = 1;
        let gas_price = 2; // price > amount
        let gas_limit = 0; // don't include any gas execution fees
        let factor = 1;
        let params = ConsensusParameters::default().with_gas_price_factor(factor);

        let transaction = base_asset_tx(rng, input_amount, gas_price, gas_limit);

        let err = transaction
            .into_checked(0, &params, &Default::default())
            .expect_err("insufficient fee amount expected");

        let provided = match err {
            CheckError::InsufficientFeeAmount { provided, .. } => provided,
            _ => panic!("expected insufficient fee amount; found {err:?}"),
        };

        assert_eq!(provided, input_amount);
    }

    #[test]
    fn checked_tx_fails_when_provided_fees_dont_cover_gas_costs() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let input_amount = 10;
        let factor = 1;
        let params = ConsensusParameters::default().with_gas_price_factor(factor);
        // make gas price too high for the input amount
        let gas_price = 1;
        let gas_limit = input_amount + 1; // make gas cost 1 higher than input amount

        let transaction = base_asset_tx(rng, input_amount, gas_price, gas_limit);

        let err = transaction
            .into_checked(0, &params, &Default::default())
            .expect_err("insufficient fee amount expected");

        let provided = match err {
            CheckError::InsufficientFeeAmount { provided, .. } => provided,
            _ => panic!("expected insufficient fee amount; found {err:?}"),
        };

        assert_eq!(provided, input_amount);
    }

    #[test]
    fn bytes_fee_cant_overflow() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let input_amount = 1000;
        let gas_price = Word::MAX;
        let gas_limit = 0; // ensure only bytes are included in fee
        let params = ConsensusParameters::default().with_gas_price_factor(1);
        let transaction = base_asset_tx(rng, input_amount, gas_price, gas_limit);

        let err = transaction
            .into_checked(0, &params, &Default::default())
            .expect_err("overflow expected");

        assert_eq!(err, CheckError::ArithmeticOverflow);
    }

    #[test]
    fn gas_fee_cant_overflow() {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 1000;
        let gas_price = Word::MAX;
        let gas_limit = 2; // 2 * max should cause gas fee overflow
        let params = ConsensusParameters::default().with_gas_price_factor(1);

        let transaction = base_asset_tx(rng, input_amount, gas_price, gas_limit);

        let err = transaction
            .into_checked(0, &params, &Default::default())
            .expect_err("overflow expected");

        assert_eq!(err, CheckError::ArithmeticOverflow);
    }

    #[test]
    fn checked_tx_fails_if_asset_is_overspent_by_coin_output() {
        let input_amount = 1_000;
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let secret = SecretKey::random(rng);
        let any_asset = rng.gen();
        let tx = TransactionBuilder::script(vec![], vec![])
            .gas_price(1)
            .gas_limit(100)
            // base asset
            .add_unsigned_coin_input(secret, rng.gen(), input_amount, AssetId::default(), rng.gen(), 0)
            .add_output(Output::change(rng.gen(), 0, AssetId::default()))
            // arbitrary spending asset
            .add_unsigned_coin_input(secret, rng.gen(), input_amount, any_asset, rng.gen(), 0)
            .add_output(Output::coin(rng.gen(), input_amount + 1, any_asset))
            .add_output(Output::change(rng.gen(), 0, any_asset))
            .finalize();

        let checked = tx
            .into_checked(0, &ConsensusParameters::DEFAULT, &Default::default())
            .expect_err("Expected valid transaction");

        assert_eq!(
            CheckError::InsufficientInputAmount {
                asset: any_asset,
                expected: input_amount + 1,
                provided: input_amount
            },
            checked
        );
    }

    #[test]
    fn basic_check_marks_basic_flag() {
        let block_height = 1;
        let params = ConsensusParameters::default();

        let tx = Transaction::default();
        // Sets Checks::Basic
        let checked = tx.into_checked_basic(block_height, &params).unwrap();
        assert!(checked.checks().contains(Checks::Basic));
    }

    #[test]
    fn signatures_check_marks_signatures_flag() {
        let mut rng = StdRng::seed_from_u64(1);
        let block_height = 1;
        let params = ConsensusParameters::default();

        let tx = valid_coin_tx(&mut rng, 1, 100000, 1000000, 10);
        let checked = tx
            // Sets Checks::Basic
            .into_checked_basic(block_height, &params)
            .unwrap()
            // Sets Checks::Signatures
            .check_signatures()
            .unwrap();

        assert!(checked.checks().contains(Checks::Basic | Checks::Signatures));
    }

    #[test]
    fn predicates_check_marks_predicate_flag() {
        let mut rng = StdRng::seed_from_u64(1);
        let block_height = 1;
        let params = ConsensusParameters::default();
        let gas_costs = GasCosts::default();

        let tx = predicate_tx(&mut rng, 1, 1000000, 1000000);
        let checked = tx
            // Sets Checks::Basic
            .into_checked_basic(block_height, &params)
            .unwrap()
            // Sets Checks::Predicates
            .check_predicates(&params, &gas_costs)
            .unwrap();
        assert!(checked.checks().contains(Checks::Basic | Checks::Predicates));
    }

    fn is_valid_max_fee<Tx>(tx: &Tx, params: &ConsensusParameters) -> Result<bool, CheckError>
    where
        Tx: Chargeable + field::Inputs + field::Outputs,
    {
        let available_balances = balances::initial_free_balances(tx, params)?;
        // cant overflow as metered bytes * gas_per_byte < u64::MAX
        let bytes = (tx.metered_bytes_size() as u128) * params.gas_per_byte as u128 * tx.price() as u128;
        let gas = tx.limit() as u128 * tx.price() as u128;
        let total = bytes + gas;
        // use different division mechanism than impl
        let fee = total / params.gas_price_factor as u128;
        let fee_remainder = (total.rem_euclid(params.gas_price_factor as u128) > 0) as u128;
        let rounded_fee = (fee + fee_remainder) as u64;

        Ok(rounded_fee == available_balances.fee.total())
    }

    fn is_valid_min_fee<Tx>(tx: &Tx, params: &ConsensusParameters) -> Result<bool, CheckError>
    where
        Tx: Chargeable + field::Inputs + field::Outputs,
    {
        let available_balances = balances::initial_free_balances(tx, params)?;
        // cant overflow as metered bytes * gas_per_byte < u64::MAX
        let bytes = (tx.metered_bytes_size() as u128) * params.gas_per_byte as u128 * tx.price() as u128;
        // use different division mechanism than impl
        let fee = bytes / params.gas_price_factor as u128;
        let fee_remainder = (bytes.rem_euclid(params.gas_price_factor as u128) > 0) as u128;
        let rounded_fee = (fee + fee_remainder) as u64;

        Ok(rounded_fee == available_balances.fee.bytes())
    }

    fn valid_coin_tx(
        rng: &mut StdRng,
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        output_amount: u64,
    ) -> Script {
        let asset = AssetId::default();
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_unsigned_coin_input(rng.gen(), rng.gen(), input_amount, asset, rng.gen(), 0)
            .add_input(Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()))
            .add_output(Output::contract(1, rng.gen(), rng.gen()))
            .add_output(Output::coin(rng.gen(), output_amount, asset))
            .add_output(Output::change(rng.gen(), 0, asset))
            .finalize()
    }

    // used when proptesting to avoid expensive crypto signatures
    fn predicate_tx(rng: &mut StdRng, gas_price: u64, gas_limit: u64, fee_input_amount: u64) -> Script {
        let asset = AssetId::default();
        let predicate = vec![op::ret(1)].into_iter().collect::<Vec<u8>>();
        let owner = Input::predicate_owner(&predicate);
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_input(Input::coin_predicate(
                rng.gen(),
                owner,
                fee_input_amount,
                asset,
                rng.gen(),
                0,
                predicate,
                vec![],
            ))
            .add_output(Output::change(rng.gen(), 0, asset))
            .finalize()
    }

    // used to verify message inputs can cover fees
    fn signed_message_tx(
        rng: &mut StdRng,
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        output_amount: u64,
    ) -> Script {
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_unsigned_message_input(rng.gen(), rng.gen(), rng.gen(), input_amount, vec![])
            .add_output(Output::message(rng.gen(), output_amount))
            .finalize()
    }

    fn predicate_message_tx(
        rng: &mut StdRng,
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        output_amount: u64,
    ) -> Script {
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_input(Input::message_predicate(
                rng.gen(),
                rng.gen(),
                rng.gen(),
                input_amount,
                rng.gen(),
                vec![],
                vec![],
                vec![],
            ))
            .add_output(Output::message(rng.gen(), output_amount))
            .finalize()
    }

    fn base_asset_tx(rng: &mut StdRng, input_amount: u64, gas_price: u64, gas_limit: u64) -> Script {
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_unsigned_coin_input(rng.gen(), rng.gen(), input_amount, AssetId::default(), rng.gen(), 0)
            .add_output(Output::change(rng.gen(), 0, AssetId::default()))
            .finalize()
    }
}

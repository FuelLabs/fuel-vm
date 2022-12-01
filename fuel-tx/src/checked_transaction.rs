//! A checked transaction is type-wrapper for transactions which have been checked.
//! It is impossible to construct a checked transaction without performing necessary checks.
//!
//! This allows the VM to accept transactions with metadata that have been already verified upstream.

#![allow(non_upper_case_globals)]
use crate::{
    field, Chargeable, CheckError, Checkable, ConsensusParameters, Create, Input, Mint, Output,
    Script, Transaction, TransactionFee,
};
use fuel_types::{AssetId, Word};

use alloc::collections::BTreeMap;
use core::borrow::Borrow;

bitflags::bitflags! {
    /// Possible types of transaction checks.
    pub struct Checks: u32 {
        /// Basic checks defined in the specification for each transaction:
        /// https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/tx_format.md#transaction
        const Basic         = 0b00000001;
        /// Check that signature in the transactions are valid.
        const Signatures    = 0b00000010;
        /// All possible checks.
        const All           = Self::Basic.bits
                            | Self::Signatures.bits;
    }
}

impl core::fmt::Display for Checks {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:032b}", self.bits)
    }
}

/// The type describes that the inner transaction was already checked.
///
/// All fields are private, and there is no constructor, so it is impossible to create the instance
/// of `Checked` outside the `fuel-tx` crate.
///
/// The inner data is immutable to prevent modification to invalidate the checking.
///
/// If you need to modify an inner state, you need to get inner values
/// (via the `Into<(Tx, Tx ::Metadata)>` trait), modify them and check again.
///
/// # Dev note: Avoid serde serialization of this type. Since checked tx would need to be
/// re-validated on deserialization anyways, it's cleaner to redo the tx check.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Checked<Tx: IntoChecked> {
    transaction: Tx,
    metadata: Tx::Metadata,
    checks_bitmask: Checks,
}

impl<Tx: IntoChecked> Checked<Tx> {
    fn new(transaction: Tx, metadata: Tx::Metadata, checks_bitmask: Checks) -> Self {
        Checked {
            transaction,
            metadata,
            checks_bitmask,
        }
    }

    pub(crate) fn basic(transaction: Tx, metadata: Tx::Metadata) -> Self {
        Checked::new(transaction, metadata, Checks::Basic)
    }

    /// Returns reference on inner transaction.
    pub fn transaction(&self) -> &Tx {
        &self.transaction
    }

    /// Returns the metadata generated during the check for transaction.
    pub fn metadata(&self) -> &Tx::Metadata {
        &self.metadata
    }

    /// Returns the bitmask of all passed checks.
    pub fn checks(&self) -> &Checks {
        &self.checks_bitmask
    }

    /// Performs check of signatures, if not yet.
    pub fn check_signatures(mut self) -> Result<Self, CheckError> {
        if !self.checks_bitmask.contains(Checks::Signatures) {
            self.transaction.check_signatures()?;
            self.checks_bitmask.insert(Checks::Signatures);
        }
        Ok(self)
    }
}

#[cfg(feature = "internals")]
impl<Tx: IntoChecked + Default> Default for Checked<Tx> {
    fn default() -> Self {
        Tx::default()
            .into_checked(Default::default(), &Default::default())
            .expect("default tx should produce a valid fully checked transaction")
    }
}

impl<Tx: IntoChecked> From<Checked<Tx>> for (Tx, Tx::Metadata) {
    fn from(checked: Checked<Tx>) -> Self {
        let Checked {
            transaction,
            metadata,
            ..
        } = checked;

        (transaction, metadata)
    }
}

impl<Tx: IntoChecked> AsRef<Tx> for Checked<Tx> {
    fn as_ref(&self) -> &Tx {
        &self.transaction
    }
}

#[cfg(feature = "internals")]
impl<Tx: IntoChecked> AsMut<Tx> for Checked<Tx> {
    fn as_mut(&mut self) -> &mut Tx {
        &mut self.transaction
    }
}

impl<Tx: IntoChecked> Borrow<Tx> for Checked<Tx> {
    fn borrow(&self) -> &Tx {
        self.transaction()
    }
}

pub trait IntoChecked: Checkable + Sized {
    /// Metadata produced during the check.
    type Metadata: Sized;

    /// Returns transaction that passed all `Checks`.
    fn into_checked(
        self,
        block_height: Word,
        params: &ConsensusParameters,
    ) -> Result<Checked<Self>, CheckError> {
        self.into_checked_basic(block_height, params)?
            .check_signatures()
    }

    /// Returns transaction that passed only `Checks::Basic`.
    fn into_checked_basic(
        self,
        block_height: Word,
        params: &ConsensusParameters,
    ) -> Result<Checked<Self>, CheckError>;
}

/// The Enum version of `Checked<Transaction>` allows getting the inner variant without losing
/// "checked" status.
///
/// It is possible to freely convert `Checked<Transaction>` into `CheckedTransaction` and vice
/// verse without the overhead.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum CheckedTransaction {
    Script(Checked<Script>),
    Create(Checked<Create>),
    Mint(Checked<Mint>),
}

impl From<Checked<Transaction>> for CheckedTransaction {
    fn from(checked: Checked<Transaction>) -> Self {
        let Checked {
            transaction,
            metadata,
            checks_bitmask,
        } = checked;

        // # Dev note: Avoid wildcard pattern to be sure that all variants are covered.
        match (transaction, metadata) {
            (Transaction::Script(transaction), CheckedMetadata::Script(metadata)) => {
                Self::Script(Checked::new(transaction, metadata, checks_bitmask))
            }
            (Transaction::Create(transaction), CheckedMetadata::Create(metadata)) => {
                Self::Create(Checked::new(transaction, metadata, checks_bitmask))
            }
            (Transaction::Mint(transaction), CheckedMetadata::Mint(metadata)) => {
                Self::Mint(Checked::new(transaction, metadata, checks_bitmask))
            }
            // The code should produce the `CheckedMetadata` for the corresponding transaction
            // variant. It is done in the implementation of the `IntoChecked` trait for
            // `Transaction`. With the current implementation, the patterns below are unreachable.
            (Transaction::Script(_), _) => unreachable!(),
            (Transaction::Create(_), _) => unreachable!(),
            (Transaction::Mint(_), _) => unreachable!(),
        }
    }
}

impl From<Checked<Script>> for CheckedTransaction {
    fn from(checked: Checked<Script>) -> Self {
        Self::Script(checked)
    }
}

impl From<Checked<Create>> for CheckedTransaction {
    fn from(checked: Checked<Create>) -> Self {
        Self::Create(checked)
    }
}

impl From<Checked<Mint>> for CheckedTransaction {
    fn from(checked: Checked<Mint>) -> Self {
        Self::Mint(checked)
    }
}

impl From<CheckedTransaction> for Checked<Transaction> {
    fn from(checked: CheckedTransaction) -> Self {
        match checked {
            CheckedTransaction::Script(Checked {
                transaction,
                metadata,
                checks_bitmask,
            }) => Checked::new(transaction.into(), metadata.into(), checks_bitmask),
            CheckedTransaction::Create(Checked {
                transaction,
                metadata,
                checks_bitmask,
            }) => Checked::new(transaction.into(), metadata.into(), checks_bitmask),
            CheckedTransaction::Mint(Checked {
                transaction,
                metadata,
                checks_bitmask,
            }) => Checked::new(transaction.into(), metadata.into(), checks_bitmask),
        }
    }
}

/// The `IntoChecked` metadata for `CheckedTransaction`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum CheckedMetadata {
    Script(<Script as IntoChecked>::Metadata),
    Create(<Create as IntoChecked>::Metadata),
    Mint(<Mint as IntoChecked>::Metadata),
}

impl From<<Script as IntoChecked>::Metadata> for CheckedMetadata {
    fn from(metadata: <Script as IntoChecked>::Metadata) -> Self {
        Self::Script(metadata)
    }
}

impl From<<Create as IntoChecked>::Metadata> for CheckedMetadata {
    fn from(metadata: <Create as IntoChecked>::Metadata) -> Self {
        Self::Create(metadata)
    }
}

impl From<<Mint as IntoChecked>::Metadata> for CheckedMetadata {
    fn from(metadata: <Mint as IntoChecked>::Metadata) -> Self {
        Self::Mint(metadata)
    }
}

impl IntoChecked for Transaction {
    type Metadata = CheckedMetadata;

    fn into_checked_basic(
        self,
        block_height: Word,
        params: &ConsensusParameters,
    ) -> Result<Checked<Self>, CheckError> {
        let (transaction, metadata) = match self {
            Transaction::Script(script) => {
                let (transaction, metadata) =
                    script.into_checked_basic(block_height, params)?.into();
                (transaction.into(), metadata.into())
            }
            Transaction::Create(create) => {
                let (transaction, metadata) =
                    create.into_checked_basic(block_height, params)?.into();
                (transaction.into(), metadata.into())
            }
            Transaction::Mint(mint) => {
                let (transaction, metadata) = mint.into_checked_basic(block_height, params)?.into();
                (transaction.into(), metadata.into())
            }
        };

        Ok(Checked::basic(transaction, metadata))
    }
}

pub(crate) fn initial_free_balances<T>(
    transaction: &T,
    params: &ConsensusParameters,
) -> Result<AvailableBalances, CheckError>
where
    T: Chargeable + field::Inputs + field::Outputs,
{
    let mut balances = BTreeMap::<AssetId, Word>::new();

    // Add up all the inputs for each asset ID
    for (asset_id, amount) in transaction.inputs().iter().filter_map(|input| match input {
        // Sum coin inputs
        Input::CoinPredicate {
            asset_id, amount, ..
        }
        | Input::CoinSigned {
            asset_id, amount, ..
        } => Some((*asset_id, amount)),
        // Sum message inputs
        Input::MessagePredicate { amount, .. } | Input::MessageSigned { amount, .. } => {
            Some((AssetId::BASE, amount))
        }
        _ => None,
    }) {
        *balances.entry(asset_id).or_default() += amount;
    }

    // Deduct fee from base asset
    let fee = TransactionFee::checked_from_tx(params, transaction)
        .ok_or(CheckError::ArithmeticOverflow)?;

    let base_asset_balance = balances.entry(AssetId::BASE).or_default();

    *base_asset_balance =
        fee.checked_deduct_total(*base_asset_balance)
            .ok_or(CheckError::InsufficientFeeAmount {
                expected: fee.total(),
                provided: *base_asset_balance,
            })?;

    // reduce free balances by coin outputs
    for (asset_id, amount) in transaction
        .outputs()
        .iter()
        .filter_map(|output| match output {
            Output::Coin {
                asset_id, amount, ..
            } => Some((asset_id, amount)),
            _ => None,
        })
    {
        let balance = balances
            .get_mut(asset_id)
            .ok_or(CheckError::TransactionOutputCoinAssetIdNotFound(*asset_id))?;
        *balance = balance
            .checked_sub(*amount)
            .ok_or(CheckError::InsufficientInputAmount {
                asset: *asset_id,
                expected: *amount,
                provided: *balance,
            })?;
    }

    Ok(AvailableBalances {
        initial_free_balances: balances,
        fee,
    })
}

pub(crate) struct AvailableBalances {
    pub initial_free_balances: BTreeMap<AssetId, Word>,
    pub fee: TransactionFee,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CheckError, Script, TransactionBuilder};
    use fuel_crypto::SecretKey;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn checked_tx_has_default() {
        let height = 1;

        Checked::<Transaction>::default()
            .transaction()
            .check(height, &Default::default())
            .expect("default checked tx should be valid");
    }

    #[test]
    fn checked_tx_accepts_valid_tx() {
        // simple smoke test that valid txs can be checked
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_price = 10;
        let gas_limit = 1000;
        let input_amount = 1000;
        let output_amount = 10;
        let tx = valid_coin_tx(rng, gas_price, gas_limit, input_amount, output_amount);

        let checked = tx
            .clone()
            .into_checked(0, &ConsensusParameters::DEFAULT)
            .expect("Expected valid transaction");

        // verify transaction getter works
        assert_eq!(checked.transaction(), &tx);
        // verify available balance was decreased by max fee
        assert_eq!(
            checked.metadata().initial_free_balances[&AssetId::default()],
            input_amount - checked.metadata().fee.total() - output_amount
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
            .into_checked(0, &ConsensusParameters::DEFAULT)
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
            .into_checked(0, &ConsensusParameters::DEFAULT)
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
            .add_input(Input::contract(
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
            ))
            .add_output(Output::contract(1, rng.gen(), rng.gen()))
            .add_output(Output::coin(rng.gen(), 10, asset))
            .add_output(Output::change(rng.gen(), 0, asset))
            .add_witness(Default::default())
            .finalize();

        let checked = tx
            .into_checked(0, &ConsensusParameters::DEFAULT)
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
            .into_checked(0, &params)
            .expect_err("insufficient fee amount expected");

        let provided = match err {
            CheckError::InsufficientFeeAmount { provided, .. } => provided,
            _ => panic!("expected insufficient fee amount; found {:?}", err),
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
            .into_checked(0, &params)
            .expect_err("insufficient fee amount expected");

        let provided = match err {
            CheckError::InsufficientFeeAmount { provided, .. } => provided,
            _ => panic!("expected insufficient fee amount; found {:?}", err),
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
            .into_checked(0, &params)
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
            .into_checked(0, &params)
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
            .add_unsigned_coin_input(
                secret,
                rng.gen(),
                input_amount,
                AssetId::default(),
                rng.gen(),
                0,
            )
            .add_output(Output::change(rng.gen(), 0, AssetId::default()))
            // arbitrary spending asset
            .add_unsigned_coin_input(secret, rng.gen(), input_amount, any_asset, rng.gen(), 0)
            .add_output(Output::coin(rng.gen(), input_amount + 1, any_asset))
            .add_output(Output::change(rng.gen(), 0, any_asset))
            .finalize();

        let checked = tx
            .into_checked(0, &ConsensusParameters::DEFAULT)
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

    fn is_valid_max_fee<Tx>(tx: &Tx, params: &ConsensusParameters) -> Result<bool, CheckError>
    where
        Tx: Chargeable + field::Inputs + field::Outputs,
    {
        let available_balances = initial_free_balances(tx, params)?;
        // cant overflow as metered bytes * gas_per_byte < u64::MAX
        let bytes =
            (tx.metered_bytes_size() as u128) * params.gas_per_byte as u128 * tx.price() as u128;
        let gas = tx.limit() as u128 * tx.price() as u128;
        let total = bytes + gas;
        // use different division mechanism than impl
        let fee = total / params.gas_price_factor as u128;
        let fee_remainder = (total.rem_euclid(params.gas_price_factor as u128) > 0) as u128;
        let rounded_fee = (fee + fee_remainder) as u64;

        Ok(rounded_fee == available_balances.fee.total)
    }

    fn is_valid_min_fee<Tx>(tx: &Tx, params: &ConsensusParameters) -> Result<bool, CheckError>
    where
        Tx: Chargeable + field::Inputs + field::Outputs,
    {
        let available_balances = initial_free_balances(tx, params)?;
        // cant overflow as metered bytes * gas_per_byte < u64::MAX
        let bytes =
            (tx.metered_bytes_size() as u128) * params.gas_per_byte as u128 * tx.price() as u128;
        // use different division mechanism than impl
        let fee = bytes / params.gas_price_factor as u128;
        let fee_remainder = (bytes.rem_euclid(params.gas_price_factor as u128) > 0) as u128;
        let rounded_fee = (fee + fee_remainder) as u64;

        Ok(rounded_fee == available_balances.fee.bytes)
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
            .add_input(Input::contract(
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
            ))
            .add_output(Output::contract(1, rng.gen(), rng.gen()))
            .add_output(Output::coin(rng.gen(), output_amount, asset))
            .add_output(Output::change(rng.gen(), 0, asset))
            .finalize()
    }

    // used when proptesting to avoid expensive crypto signatures
    fn predicate_tx(
        rng: &mut StdRng,
        gas_price: u64,
        gas_limit: u64,
        fee_input_amount: u64,
    ) -> Script {
        let asset = AssetId::default();
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_input(Input::coin_predicate(
                rng.gen(),
                rng.gen(),
                fee_input_amount,
                asset,
                rng.gen(),
                0,
                vec![],
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

    fn base_asset_tx(
        rng: &mut StdRng,
        input_amount: u64,
        gas_price: u64,
        gas_limit: u64,
    ) -> Script {
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_unsigned_coin_input(
                rng.gen(),
                rng.gen(),
                input_amount,
                AssetId::default(),
                rng.gen(),
                0,
            )
            .add_output(Output::change(rng.gen(), 0, AssetId::default()))
            .finalize()
    }
}

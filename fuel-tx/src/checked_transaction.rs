//! A checked transaction is type-wrapper for transactions which have been validated.
//! It is impossible to construct a checked transaction without performing necessary validation.
//!
//! This allows the VM to accept transactions that have been already verified upstream,
//! and consolidates logic around fee calculations and free balances.

use crate::{ConsensusParameters, Input, Output, Transaction, TransactionFee, ValidationError};

use alloc::collections::BTreeMap;
use fuel_types::{AssetId, Word};

use core::borrow::Borrow;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
// Avoid serde serialization of this type. Since checked tx would need to be re-validated on
// deserialization anyways, it's cleaner to redo the tx check.
pub struct CheckedTransaction {
    /// The transaction that was validated
    transaction: Transaction,
    /// The mapping of initial free balances
    initial_free_balances: BTreeMap<AssetId, Word>,
    /// The block height this tx was verified with
    block_height: Word,
    /// Max potential fee
    max_fee: Word,
    /// Min guaranteed fee
    min_fee: Word,
    /// Signatures verified
    checked_signatures: bool,
}

impl CheckedTransaction {
    /// Fully verify transaction, including signatures.
    pub fn check(
        transaction: Transaction,
        block_height: Word,
        params: &ConsensusParameters,
    ) -> Result<Self, ValidationError> {
        let mut checked_tx = Self::check_unsigned(transaction, block_height, params)?;
        checked_tx.transaction.validate_input_signature()?;
        checked_tx.checked_signatures = true;
        Ok(checked_tx)
    }

    /// Verify transaction, without signature checks.
    pub fn check_unsigned(
        transaction: Transaction,
        block_height: Word,
        params: &ConsensusParameters,
    ) -> Result<Self, ValidationError> {
        // fully validate transaction (with signature)
        transaction.validate_without_signature(block_height, params)?;
        // validate fees and compute free balances
        let AvailableBalances {
            initial_free_balances,
            max_fee,
            min_fee,
        } = Self::_initial_free_balances(&transaction, params)?;

        Ok(CheckedTransaction {
            transaction,
            initial_free_balances,
            block_height,
            max_fee,
            min_fee,
            checked_signatures: false,
        })
    }

    pub const fn transaction(&self) -> &Transaction {
        &self.transaction
    }

    // TODO: const blocked by https://github.com/rust-lang/rust/issues/92476
    pub fn free_balances(&self) -> impl Iterator<Item = (&AssetId, &Word)> {
        self.initial_free_balances.iter()
    }

    pub const fn block_height(&self) -> Word {
        self.block_height
    }

    pub const fn max_fee(&self) -> Word {
        self.max_fee
    }

    pub const fn min_fee(&self) -> Word {
        self.min_fee
    }

    pub const fn checked_signatures(&self) -> bool {
        self.checked_signatures
    }

    fn _initial_free_balances(
        transaction: &Transaction,
        params: &ConsensusParameters,
    ) -> Result<AvailableBalances, ValidationError> {
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
                Some((AssetId::default(), amount))
            }
            _ => None,
        }) {
            *balances.entry(asset_id).or_default() += amount;
        }

        // Deduct fee from base asset

        let fee = TransactionFee::checked_from_tx(params, transaction)
            .ok_or(ValidationError::ArithmeticOverflow)?;

        let base_asset = AssetId::default();
        let base_asset_balance = balances.entry(base_asset).or_default();

        *base_asset_balance = fee.checked_deduct_total(*base_asset_balance).ok_or(
            ValidationError::InsufficientFeeAmount {
                expected: fee.total(),
                provided: *base_asset_balance,
            },
        )?;

        let (min_fee, max_fee) = fee.into_inner();

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
            let balance = balances.get_mut(asset_id).ok_or(
                ValidationError::TransactionOutputCoinAssetIdNotFound(*asset_id),
            )?;
            *balance =
                balance
                    .checked_sub(*amount)
                    .ok_or(ValidationError::InsufficientInputAmount {
                        asset: *asset_id,
                        expected: *amount,
                        provided: *balance,
                    })?;
        }

        Ok(AvailableBalances {
            initial_free_balances: balances,
            max_fee,
            min_fee,
        })
    }
}

struct AvailableBalances {
    initial_free_balances: BTreeMap<AssetId, Word>,
    max_fee: Word,
    min_fee: Word,
}

impl AsRef<Transaction> for CheckedTransaction {
    fn as_ref(&self) -> &Transaction {
        &self.transaction
    }
}

impl Borrow<Transaction> for CheckedTransaction {
    fn borrow(&self) -> &Transaction {
        &self.transaction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TransactionBuilder, ValidationError};
    use fuel_crypto::SecretKey;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn checked_tx_accepts_valid_tx() {
        // simple smoke test that valid txs can be checked
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_price = 10;
        let gas_limit = 1000;
        let input_amount = 1000;
        let output_amount = 10;
        let tx = valid_coin_tx(rng, gas_price, gas_limit, input_amount, output_amount);

        let checked = CheckedTransaction::check(tx.clone(), 0, &ConsensusParameters::DEFAULT)
            .expect("Expected valid transaction");

        // verify transaction getter works
        assert_eq!(checked.transaction(), &tx);
        // verify available balance was decreased by max fee
        assert_eq!(
            checked.initial_free_balances[&AssetId::default()],
            input_amount - checked.max_fee - output_amount
        );
    }

    #[test]
    fn checked_tx_accepts_valid_signed_message_input_fees() {
        // simple test to ensure a tx that only has a message input can cover fees
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 100;
        let output_amont = 0;
        let gas_price = 100;
        let gas_limit = 1000;
        let tx = signed_message_tx(rng, gas_price, gas_limit, input_amount, output_amont);

        let checked = CheckedTransaction::check(tx, 0, &ConsensusParameters::DEFAULT)
            .expect("Expected valid transaction");

        // verify available balance was decreased by max fee
        assert_eq!(
            checked.initial_free_balances[&AssetId::default()],
            input_amount - checked.max_fee
        );
    }

    #[test]
    fn checked_tx_excludes_message_output_amount_from_fee() {
        // ensure message outputs aren't deducted from available balance
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 100;
        // set a large message output amount
        let output_amont = u64::MAX;
        let gas_price = 100;
        let gas_limit = 1000;
        let tx = signed_message_tx(rng, gas_price, gas_limit, input_amount, output_amont);

        let checked = CheckedTransaction::check(tx, 0, &ConsensusParameters::DEFAULT)
            .expect("Expected valid transaction");

        // verify available balance was decreased by max fee
        assert_eq!(
            checked.initial_free_balances[&AssetId::default()],
            input_amount - checked.max_fee
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
                0,
                0,
            ))
            .add_input(Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
            .add_output(Output::contract(1, rng.gen(), rng.gen()))
            .add_output(Output::coin(rng.gen(), 10, asset))
            .add_output(Output::change(rng.gen(), 0, asset))
            .add_witness(Default::default())
            .finalize();

        let checked = CheckedTransaction::check(tx, 0, &ConsensusParameters::DEFAULT)
            .expect_err("Expected invalid transaction");

        // assert that tx without base input assets fails
        assert_eq!(
            ValidationError::InsufficientFeeAmount {
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

        let err = CheckedTransaction::check(transaction, 0, &params)
            .expect_err("insufficient fee amount expected");

        let provided = match err {
            ValidationError::InsufficientFeeAmount { provided, .. } => provided,
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

        let err = CheckedTransaction::check(transaction, 0, &params)
            .expect_err("insufficient fee amount expected");

        let provided = match err {
            ValidationError::InsufficientFeeAmount { provided, .. } => provided,
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

        let err =
            CheckedTransaction::check(transaction, 0, &params).expect_err("overflow expected");

        assert_eq!(err, ValidationError::ArithmeticOverflow);
    }

    #[test]
    fn gas_fee_cant_overflow() {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 1000;
        let gas_price = Word::MAX;
        let gas_limit = 2; // 2 * max should cause gas fee overflow
        let params = ConsensusParameters::default().with_gas_price_factor(1);

        let transaction = base_asset_tx(rng, input_amount, gas_price, gas_limit);

        let err =
            CheckedTransaction::check(transaction, 0, &params).expect_err("overflow expected");

        assert_eq!(err, ValidationError::ArithmeticOverflow);
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
            .add_unsigned_coin_input(&secret, rng.gen(), input_amount, AssetId::default(), 0)
            .add_output(Output::change(rng.gen(), 0, AssetId::default()))
            // arbitrary spending asset
            .add_unsigned_coin_input(&secret, rng.gen(), input_amount, any_asset, 0)
            .add_output(Output::coin(rng.gen(), input_amount + 1, any_asset))
            .add_output(Output::change(rng.gen(), 0, any_asset))
            .finalize();

        let checked = CheckedTransaction::check(tx, 0, &ConsensusParameters::DEFAULT)
            .expect_err("Expected valid transaction");

        assert_eq!(
            ValidationError::InsufficientInputAmount {
                asset: any_asset,
                expected: input_amount + 1,
                provided: input_amount
            },
            checked
        );
    }

    fn is_valid_max_fee(
        tx: &Transaction,
        params: &ConsensusParameters,
    ) -> Result<bool, ValidationError> {
        let available_balances = CheckedTransaction::_initial_free_balances(tx, params)?;
        // cant overflow as metered bytes * gas_per_byte < u64::MAX
        let bytes = (tx.metered_bytes_size() as u128)
            * params.gas_per_byte as u128
            * tx.gas_price() as u128;
        let gas = tx.gas_limit() as u128 * tx.gas_price() as u128;
        let total = bytes + gas;
        // use different division mechanism than impl
        let fee = total / params.gas_price_factor as u128;
        let fee_remainder = (total.rem_euclid(params.gas_price_factor as u128) > 0) as u128;
        let rounded_fee = (fee + fee_remainder) as u64;

        Ok(rounded_fee == available_balances.max_fee)
    }

    fn is_valid_min_fee(
        tx: &Transaction,
        params: &ConsensusParameters,
    ) -> Result<bool, ValidationError> {
        let available_balances = CheckedTransaction::_initial_free_balances(tx, params)?;
        // cant overflow as metered bytes * gas_per_byte < u64::MAX
        let bytes = (tx.metered_bytes_size() as u128)
            * params.gas_per_byte as u128
            * tx.gas_price() as u128;
        // use different division mechanism than impl
        let fee = bytes / params.gas_price_factor as u128;
        let fee_remainder = (bytes.rem_euclid(params.gas_price_factor as u128) > 0) as u128;
        let rounded_fee = (fee + fee_remainder) as u64;

        Ok(rounded_fee == available_balances.min_fee)
    }

    fn valid_coin_tx(
        rng: &mut StdRng,
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        output_amount: u64,
    ) -> Transaction {
        let asset = AssetId::default();
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_unsigned_coin_input(&rng.gen(), rng.gen(), input_amount, asset, 0)
            .add_input(Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
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
    ) -> Transaction {
        let asset = AssetId::default();
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_input(Input::coin_predicate(
                rng.gen(),
                rng.gen(),
                fee_input_amount,
                asset,
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
    ) -> Transaction {
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_unsigned_message_input(
                &rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                input_amount,
                vec![],
            )
            .add_output(Output::message(rng.gen(), output_amount))
            .finalize()
    }

    fn predicate_message_tx(
        rng: &mut StdRng,
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        output_amount: u64,
    ) -> Transaction {
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_input(Input::message_predicate(
                rng.gen(),
                rng.gen(),
                rng.gen(),
                input_amount,
                rng.gen(),
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
    ) -> Transaction {
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_unsigned_coin_input(&rng.gen(), rng.gen(), input_amount, AssetId::default(), 0)
            .add_output(Output::change(rng.gen(), 0, AssetId::default()))
            .finalize()
    }
}

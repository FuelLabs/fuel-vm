use crate::{
    field::Inputs,
    FeeParameters,
    GasCosts,
};
use fuel_asm::Word;
use itertools::Itertools;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TransactionFee {
    pub(crate) min_fee: Word,
    pub(crate) max_fee: Word,
    pub(crate) min_gas: Word,
    pub(crate) max_gas: Word,
}

impl From<TransactionFee> for Word {
    fn from(fee: TransactionFee) -> Word {
        fee.max_fee
    }
}

impl TransactionFee {
    pub const fn new(min_fee: Word, max_fee: Word, min_gas: Word, max_gas: Word) -> Self {
        Self {
            min_fee,
            max_fee,
            min_gas,
            max_gas,
        }
    }

    /// Minimum fee value to pay for the base transaction without script execution.
    pub const fn min_fee(&self) -> Word {
        self.min_fee
    }

    /// Maximum fee value to pay for the transaction with script execution.
    pub const fn max_fee(&self) -> Word {
        self.max_fee
    }

    /// The minimum amount of gas (not fee!) used by this tx
    pub const fn min_gas(&self) -> Word {
        self.min_gas
    }

    /// The max amount of gas (not fee!) usable by this tx
    pub const fn max_gas(&self) -> Word {
        self.max_gas
    }

    /// Convert into a tuple containing the inner min & total fee values
    pub const fn into_inner(self) -> (Word, Word) {
        (self.min_fee, self.max_fee)
    }

    /// Attempt to subtract the maximum fee value from a given balance
    ///
    /// Will return `None` if arithmetic overflow occurs.
    pub fn checked_deduct_total(&self, balance: Word) -> Option<Word> {
        let fee = self.max_fee();

        balance.checked_sub(fee)
    }

    /// Attempt to create a transaction fee from parameters and value arguments
    ///
    /// Will return `None` if arithmetic overflow occurs or `max_fee` less than `min_fee`.
    pub fn checked_from_values(
        params: &FeeParameters,
        metered_bytes: Word,
        gas_used_by_signature_recovery: Word,
        gas_used_by_predicates: Word,
        gas_limit: Word,
        gas_price: Word,
    ) -> Option<Self> {
        let factor = params.gas_price_factor as u128;

        // TODO: use native div_ceil once stabilized out from nightly
        let bytes_gas = params.gas_per_byte.checked_mul(metered_bytes)?;
        let min_gas = bytes_gas
            .checked_add(gas_used_by_signature_recovery)?
            .checked_add(gas_used_by_predicates)?;
        let max_gas = bytes_gas.checked_add(gas_limit)?;

        let max_gas_to_pay = max_gas.checked_mul(gas_price).and_then(|total| {
            num_integer::div_ceil(total as u128, factor).try_into().ok()
        });

        let min_gas_to_pay = min_gas.checked_mul(gas_price).and_then(|bytes| {
            num_integer::div_ceil(bytes as u128, factor).try_into().ok()
        });

        min_gas_to_pay
            .zip(max_gas_to_pay)
            .map(|(min_gas_to_pay, max_gas_to_pay)| {
                Self::new(min_gas_to_pay, max_gas_to_pay, min_gas, max_gas)
            })
    }

    /// Attempt to calculate a gas as asset value, using the price factor defined in the
    /// consensus parameters.
    ///
    /// Will return `None` if overflow occurs
    pub fn gas_refund_value(
        fee_params: &FeeParameters,
        gas: Word,
        price: Word,
    ) -> Option<Word> {
        let gas = gas as u128;
        let price = price as u128;
        let factor = fee_params.gas_price_factor as u128;

        gas.checked_mul(price)
            .map(|g| num_integer::div_floor(g, factor))
            .and_then(|g| g.try_into().ok())
    }

    /// Attempt to create a transaction fee from parameters and transaction internals
    ///
    /// Will return `None` if arithmetic overflow occurs.
    pub fn checked_from_tx<T: Chargeable + Inputs>(
        gas_costs: &GasCosts,
        params: &FeeParameters,
        tx: &T,
    ) -> Option<Self> {
        let metered_bytes = tx.metered_bytes_size() as Word;
        let unique_witnesses = tx
            .inputs()
            .iter()
            .filter_map(|input| input.witness_index())
            .sorted_unstable()
            .dedup()
            .count() as Word;
        let gas_used_by_signature_recovery =
            unique_witnesses.saturating_mul(gas_costs.ecr1);
        let gas_used_by_predicates = tx.gas_used_by_predicates();
        let gas_limit = tx.limit();
        let gas_price = tx.price();

        Self::checked_from_values(
            params,
            metered_bytes,
            gas_used_by_signature_recovery,
            gas_used_by_predicates,
            gas_limit,
            gas_price,
        )
    }
}

/// Means that the blockchain charges fee for the transaction.
pub trait Chargeable {
    /// Returns the gas price.
    fn price(&self) -> Word;

    /// Returns the gas limit.
    fn limit(&self) -> Word;

    /// Used for accounting purposes when charging byte based fees.
    fn metered_bytes_size(&self) -> usize;

    /// Used for accounting purposes when charging for predicates.
    fn gas_used_by_predicates(&self) -> Word;
}

#[cfg(test)]
mod tests {
    use crate::{
        FeeParameters,
        TransactionFee,
        Word,
    };

    const PARAMS: FeeParameters = FeeParameters::DEFAULT
        .with_gas_per_byte(2)
        .with_gas_price_factor(3);

    fn gas_to_fee(gas: u64, gas_price: Word) -> f64 {
        let fee = gas * gas_price;
        fee as f64 / PARAMS.gas_price_factor as f64
    }

    #[test]
    fn base_fee_is_calculated_correctly() {
        let metered_bytes = 5;
        let gas_used_by_signature_recovery = 13;
        let gas_used_by_predicates = 7;
        let gas_limit = 7;
        let gas_price = 11;

        let fee = TransactionFee::checked_from_values(
            &PARAMS,
            metered_bytes,
            gas_used_by_signature_recovery,
            gas_used_by_predicates,
            gas_limit,
            gas_price,
        )
        .expect("failed to calculate fee");

        let expected_max_gas = PARAMS.gas_per_byte * metered_bytes + gas_limit;
        let expected_max_fee = gas_to_fee(expected_max_gas, gas_price).ceil() as Word;
        let expected_min_gas = PARAMS.gas_per_byte * metered_bytes
            + gas_used_by_signature_recovery
            + gas_limit;
        let expected_min_fee = gas_to_fee(expected_min_gas, gas_price).ceil() as Word;

        assert_eq!(expected_max_fee, fee.max_fee);
        assert_eq!(expected_min_fee, fee.min_fee);
    }

    #[test]
    fn base_fee_ceils() {
        let metered_bytes = 5;
        let gas_used_by_signature_recovery = 12;
        let gas_used_by_predicates = 7;
        let gas_limit = 7;
        let gas_price = 11;

        let fee = TransactionFee::checked_from_values(
            &PARAMS,
            metered_bytes,
            gas_used_by_signature_recovery,
            gas_used_by_predicates,
            gas_limit,
            gas_price,
        )
        .expect("failed to calculate fee");

        let expected_max_gas = PARAMS.gas_per_byte * metered_bytes + gas_limit;
        let expected_max_fee = gas_to_fee(expected_max_gas, gas_price);
        let truncated = expected_max_fee as Word;
        let expected_max_fee = expected_max_fee.ceil() as Word;
        assert_ne!(truncated, expected_max_fee);
        assert_eq!(expected_max_fee, fee.max_fee);

        let expected_min_gas = PARAMS.gas_per_byte * metered_bytes
            + gas_used_by_signature_recovery
            + gas_limit;
        let expected_min_fee = gas_to_fee(expected_min_gas, gas_price);
        let truncated = expected_max_fee as Word;
        let expected_min_fee = expected_min_fee.ceil() as Word;
        assert_ne!(truncated, expected_min_fee);
        assert_eq!(expected_min_fee, fee.min_fee);
    }

    #[test]
    fn base_fee_zeroes() {
        let metered_bytes = 5;
        let gas_used_by_signature_recovery = 12;
        let gas_used_by_predicates = 7;
        let gas_limit = 7;
        let gas_price = 0;

        let fee = TransactionFee::checked_from_values(
            &PARAMS,
            metered_bytes,
            gas_used_by_signature_recovery,
            gas_used_by_predicates,
            gas_limit,
            gas_price,
        )
        .expect("failed to calculate fee");

        let expected = 0u64;

        assert_eq!(expected, fee.max_fee);
        assert_eq!(expected, fee.min_fee);
    }

    #[test]
    fn base_fee_wont_overflow_on_bytes() {
        let metered_bytes = Word::MAX;
        let gas_used_by_signature_recovery = 12;
        let gas_used_by_predicates = 7;
        let gas_limit = 7;
        let gas_price = 11;

        let overflow = TransactionFee::checked_from_values(
            &PARAMS,
            metered_bytes,
            gas_used_by_signature_recovery,
            gas_used_by_predicates,
            gas_limit,
            gas_price,
        )
        .is_none();

        assert!(overflow);
    }

    #[test]
    fn base_fee_wont_overflow_on_gas_used_by_predicates() {
        let metered_bytes = 5;
        let gas_used_by_signature_recovery = 12;
        let gas_used_by_predicates = Word::MAX;
        let gas_limit = 7;
        let gas_price = 11;

        let overflow = TransactionFee::checked_from_values(
            &PARAMS,
            metered_bytes,
            gas_used_by_signature_recovery,
            gas_used_by_predicates,
            gas_limit,
            gas_price,
        )
        .is_none();

        assert!(overflow);
    }

    #[test]
    fn base_fee_wont_overflow_on_limit() {
        let metered_bytes = 5;
        let gas_used_by_signature_recovery = 12;
        let gas_used_by_predicates = 7;
        let gas_limit = Word::MAX;
        let gas_price = 11;

        let overflow = TransactionFee::checked_from_values(
            &PARAMS,
            metered_bytes,
            gas_used_by_signature_recovery,
            gas_used_by_predicates,
            gas_limit,
            gas_price,
        )
        .is_none();

        assert!(overflow);
    }

    #[test]
    fn base_fee_wont_overflow_on_price() {
        let metered_bytes = 5;
        let gas_used_by_signature_recovery = 12;
        let gas_used_by_predicates = 7;
        let gas_limit = 7;
        let gas_price = Word::MAX;

        let overflow = TransactionFee::checked_from_values(
            &PARAMS,
            metered_bytes,
            gas_used_by_signature_recovery,
            gas_used_by_predicates,
            gas_limit,
            gas_price,
        )
        .is_none();

        assert!(overflow);
    }

    #[test]
    fn base_fee_gas_limit_less_than_gas_used_by_predicates() {
        let metered_bytes = 5;
        let gas_used_by_signature_recovery = 12;
        let gas_used_by_predicates = 8;
        let gas_limit = 7;
        let gas_price = 11;

        let fee = TransactionFee::checked_from_values(
            &PARAMS,
            metered_bytes,
            gas_used_by_signature_recovery,
            gas_used_by_predicates,
            gas_limit,
            gas_price,
        )
        .expect("failed to calculate fee");

        assert!(fee.min_fee > fee.max_fee);
    }
}

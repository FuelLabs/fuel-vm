use fuel_asm::Word;

use crate::ConsensusParameters;

use crate::Transaction;
use core::borrow::Borrow;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TransactionFee {
    pub(crate) bytes: Word,
    pub(crate) total: Word,
    pub(crate) min_gas: Word,
    pub(crate) max_gas: Word,
}

impl From<TransactionFee> for Word {
    fn from(fee: TransactionFee) -> Word {
        fee.total()
    }
}

impl TransactionFee {
    pub const fn new(bytes: Word, total: Word, min_gas: Word, max_gas: Word) -> Self {
        Self {
            bytes,
            total,
            min_gas,
            max_gas,
        }
    }

    /// Minimum fee value to pay for the metered bytes
    pub const fn min(&self) -> Word {
        self.bytes
    }

    /// Maximum fee value composed of metered bytes cost + tx gas limit, with price factor
    /// correction
    pub const fn total(&self) -> Word {
        self.total
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
        (self.bytes, self.total)
    }

    /// Attempt to subtract the maximum fee value from a given balance
    ///
    /// Will return `None` if arithmetic overflow occurs.
    pub fn checked_deduct_total(&self, balance: Word) -> Option<Word> {
        let fee = self.total();

        balance.checked_sub(fee)
    }

    /// Attempt to create a transaction fee from parameters and value arguments
    ///
    /// Will return `None` if arithmetic overflow occurs.
    pub fn checked_from_values(
        params: &ConsensusParameters,
        metered_bytes: Word,
        gas_limit: Word,
        gas_price: Word,
    ) -> Option<Self> {
        let factor = params.gas_price_factor as u128;

        // TODO: use native div_ceil once stabilized out from nightly
        let bytes_gas = params.gas_per_byte.checked_mul(metered_bytes)?;
        let max_gas = bytes_gas.checked_add(gas_limit)?;

        let total = max_gas
            .checked_mul(gas_price)
            .and_then(|total| num_integer::div_ceil(total as u128, factor).try_into().ok());

        let bytes = bytes_gas
            .checked_mul(gas_price)
            .and_then(|bytes| num_integer::div_ceil(bytes as u128, factor).try_into().ok());

        bytes
            .zip(total)
            .map(|(bytes, total)| Self::new(bytes, total, bytes_gas, max_gas))
    }

    /// Attempt to calculate a gas as asset value, using the price factor defined in the consensus
    /// parameters.
    ///
    /// Will return `None` if overflow occurs
    pub fn gas_refund_value(params: &ConsensusParameters, gas: Word, price: Word) -> Option<Word> {
        let gas = gas as u128;
        let price = price as u128;
        let factor = params.gas_price_factor as u128;

        gas.checked_mul(price)
            .map(|g| num_integer::div_floor(g, factor))
            .and_then(|g| g.try_into().ok())
    }

    /// Attempt to create a transaction fee from parameters and transaction internals
    ///
    /// Will return `None` if arithmetic overflow occurs.
    pub fn checked_from_tx<T>(params: &ConsensusParameters, tx: T) -> Option<Self>
    where
        T: Borrow<Transaction>,
    {
        let tx = tx.borrow();

        let metered_bytes = tx.metered_bytes_size() as Word;
        let gas_limit = tx.gas_limit();
        let gas_price = tx.gas_price();

        Self::checked_from_values(params, metered_bytes, gas_limit, gas_price)
    }
}

#[cfg(test)]
mod tests {
    use crate::{ConsensusParameters, TransactionFee, Word};

    const PARAMS: ConsensusParameters = ConsensusParameters::DEFAULT
        .with_gas_per_byte(2)
        .with_gas_price_factor(3);

    #[test]
    fn base_fee_is_calculated_correctly() {
        let metered_bytes = 5;
        let gas_limit = 7;
        let gas_price = 11;

        let fee: Word =
            TransactionFee::checked_from_values(&PARAMS, metered_bytes, gas_limit, gas_price)
                .expect("failed to calculate fee")
                .into();

        let expected = PARAMS.gas_per_byte * metered_bytes + gas_limit;
        let expected = expected * gas_price;
        let expected = expected as f64 / PARAMS.gas_price_factor as f64;
        let expected = expected.ceil() as Word;

        assert_eq!(expected, fee);
    }

    #[test]
    fn base_fee_ceils() {
        let metered_bytes = 5;
        let gas_limit = 7;
        let gas_price = 11;

        let fee: Word =
            TransactionFee::checked_from_values(&PARAMS, metered_bytes, gas_limit, gas_price)
                .expect("failed to calculate fee")
                .into();

        let expected = PARAMS.gas_per_byte * metered_bytes + gas_limit;
        let expected = expected * gas_price;
        let expected = expected as f64 / PARAMS.gas_price_factor as f64;
        let truncated = expected as Word;
        let expected = expected.ceil() as Word;

        assert_ne!(truncated, expected);
        assert_eq!(expected, fee);
    }

    #[test]
    fn base_fee_zeroes() {
        let metered_bytes = 5;
        let gas_limit = 7;
        let gas_price = 0;

        let fee: Word =
            TransactionFee::checked_from_values(&PARAMS, metered_bytes, gas_limit, gas_price)
                .expect("failed to calculate fee")
                .into();

        let expected = 0u64;

        assert_eq!(expected, fee);
    }

    #[test]
    fn base_fee_wont_overflow_on_bytes() {
        let metered_bytes = Word::MAX;
        let gas_limit = 7;
        let gas_price = 11;

        let overflow =
            TransactionFee::checked_from_values(&PARAMS, metered_bytes, gas_limit, gas_price)
                .is_none();

        assert!(overflow);
    }

    #[test]
    fn base_fee_wont_overflow_on_limit() {
        let metered_bytes = 5;
        let gas_limit = Word::MAX;
        let gas_price = 11;

        let overflow =
            TransactionFee::checked_from_values(&PARAMS, metered_bytes, gas_limit, gas_price)
                .is_none();

        assert!(overflow);
    }

    #[test]
    fn base_fee_wont_overflow_on_price() {
        let metered_bytes = 5;
        let gas_limit = 7;
        let gas_price = Word::MAX;

        let overflow =
            TransactionFee::checked_from_values(&PARAMS, metered_bytes, gas_limit, gas_price)
                .is_none();

        assert!(overflow);
    }
}

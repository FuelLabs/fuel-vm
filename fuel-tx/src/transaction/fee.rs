use crate::{
    FeeParameters,
    GasCosts,
    Input,
    field,
    field::{
        MaxFeeLimit,
        Tip,
        WitnessLimit,
    },
    input::{
        coin::{
            CoinPredicate,
            CoinSigned,
        },
        message::{
            MessageCoinPredicate,
            MessageCoinSigned,
            MessageDataPredicate,
            MessageDataSigned,
        },
    },
    policies::PolicyType,
};
use fuel_asm::Word;
use fuel_types::canonical::Serialize;
use hashbrown::HashSet;

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
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

    /// Attempt to create a transaction fee from parameters and transaction internals
    ///
    /// Will return `None` if arithmetic overflow occurs.
    pub fn checked_from_tx<T>(
        gas_costs: &GasCosts,
        params: &FeeParameters,
        tx: &T,
        gas_price: Word,
    ) -> Option<Self>
    where
        T: Chargeable,
    {
        let min_gas = tx.min_gas(gas_costs, params);
        let max_gas = tx.max_gas(gas_costs, params);
        let min_fee = tx.min_fee(gas_costs, params, gas_price).try_into().ok()?;
        let max_fee = tx.max_fee(gas_costs, params, gas_price).try_into().ok()?;

        if min_fee > max_fee {
            return None;
        }

        Some(Self::new(min_fee, max_fee, min_gas, max_gas))
    }
}

fn gas_to_fee(gas: Word, gas_price: Word, factor: Word) -> u128 {
    let total_price = (gas as u128)
        .checked_mul(gas_price as u128)
        .expect("Impossible to overflow because multiplication of two `u64` <= `u128`");
    total_price.div_ceil(factor as u128)
}

/// Returns the minimum gas required to start execution of any transaction.
pub fn min_gas<Tx>(tx: &Tx, gas_costs: &GasCosts, fee: &FeeParameters) -> Word
where
    Tx: Chargeable + ?Sized,
{
    let bytes_size = tx.metered_bytes_size();

    let vm_initialization_gas = gas_costs.vm_initialization().resolve(bytes_size as Word);

    // It's okay to saturate because we have the `max_gas_per_tx` rule for transaction
    // validity. In the production, the value always will be lower than
    // `u64::MAX`.
    let bytes_gas = fee.gas_per_byte().saturating_mul(bytes_size as u64);
    tx.gas_used_by_inputs(gas_costs)
        .saturating_add(tx.gas_used_by_metadata(gas_costs))
        .saturating_add(bytes_gas)
        .saturating_add(vm_initialization_gas)
}

/// Means that the blockchain charges fee for the transaction.
pub trait Chargeable: field::Inputs + field::Witnesses + field::Policies {
    /// Returns the minimum gas required to start transaction execution.
    fn min_gas(&self, gas_costs: &GasCosts, fee: &FeeParameters) -> Word {
        min_gas(self, gas_costs, fee)
    }

    /// Returns the maximum possible gas after the end of transaction execution.
    ///
    /// The function guarantees that the value is not less than [Self::min_gas].
    fn max_gas(&self, gas_costs: &GasCosts, fee: &FeeParameters) -> Word {
        let remaining_allowed_witness_gas = self
            .witness_limit()
            .saturating_sub(self.witnesses().size_dynamic() as u64)
            .saturating_mul(fee.gas_per_byte());

        self.min_gas(gas_costs, fee)
            .saturating_add(remaining_allowed_witness_gas)
    }

    /// Returns the minimum fee required to start transaction execution.
    fn min_fee(
        &self,
        gas_costs: &GasCosts,
        fee: &FeeParameters,
        gas_price: Word,
    ) -> u128 {
        let tip = self.tip();
        let gas_fee = gas_to_fee(
            self.min_gas(gas_costs, fee),
            gas_price,
            fee.gas_price_factor(),
        );
        gas_fee.saturating_add(tip as u128)
    }

    /// Returns the maximum possible fee after the end of transaction execution.
    ///
    /// The function guarantees that the value is not less than [Self::min_fee].
    fn max_fee(
        &self,
        gas_costs: &GasCosts,
        fee: &FeeParameters,
        gas_price: Word,
    ) -> u128 {
        let tip = self.tip();
        let gas_fee = gas_to_fee(
            self.max_gas(gas_costs, fee),
            gas_price,
            fee.gas_price_factor(),
        );
        gas_fee.saturating_add(tip as u128)
    }

    /// Returns the fee amount that can be refunded back based on the `used_gas` and
    /// current state of the transaction.
    ///
    /// Return `None` if overflow occurs.
    fn refund_fee(
        &self,
        gas_costs: &GasCosts,
        fee: &FeeParameters,
        used_gas: Word,
        gas_price: Word,
    ) -> Option<Word> {
        // We've already charged the user for witnesses as part of the minimal gas and all
        // execution required to validate transaction validity rules.
        let min_gas = self.min_gas(gas_costs, fee);

        let total_used_gas = min_gas.saturating_add(used_gas);
        let tip = self.policies().get(PolicyType::Tip).unwrap_or(0);
        let used_fee = gas_to_fee(total_used_gas, gas_price, fee.gas_price_factor())
            .saturating_add(tip as u128);

        // It is okay to saturate everywhere above because it only can decrease the value
        // of `refund`. But here, because we need to return the amount we
        // want to refund, we need to handle the overflow caused by the price.
        let used_fee: u64 = used_fee.try_into().ok()?;
        self.max_fee_limit().checked_sub(used_fee)
    }

    /// Used for accounting purposes when charging byte based fees.
    fn metered_bytes_size(&self) -> usize;

    /// Returns the gas used by the inputs.
    fn gas_used_by_inputs(&self, gas_costs: &GasCosts) -> Word;

    /// Used for accounting purposes when charging for metadata creation.
    fn gas_used_by_metadata(&self, gas_costs: &GasCosts) -> Word;

    fn has_spendable_input(&self) -> bool;
}

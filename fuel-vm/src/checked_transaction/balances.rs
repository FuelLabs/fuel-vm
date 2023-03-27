use fuel_tx::{
    field,
    input::{
        coin::{CoinPredicate, CoinSigned},
        message::{MessageCoinPredicate, MessageCoinSigned, MessageDataPredicate, MessageDataSigned},
    },
    Chargeable, CheckError, ConsensusParameters, Input, Output, TransactionFee,
};
use fuel_types::{AssetId, Word};
use std::collections::BTreeMap;

pub(crate) fn initial_free_balances<T>(
    transaction: &T,
    params: &ConsensusParameters,
) -> Result<AvailableBalances, CheckError>
where
    T: Chargeable + field::Inputs + field::Outputs,
{
    let mut non_retryable_balances = BTreeMap::<AssetId, Word>::new();
    // The sum of [`AssetId::Base`] from metadata messages.
    let mut retryable_balance: Word = 0;

    // Add up all the inputs for each asset ID
    for input in transaction.inputs().iter() {
        match input {
            // Sum coin inputs
            Input::CoinPredicate(CoinPredicate { asset_id, amount, .. })
            | Input::CoinSigned(CoinSigned { asset_id, amount, .. }) => {
                *non_retryable_balances.entry(*asset_id).or_default() += amount;
            }
            // Sum message coin inputs
            Input::MessageCoinSigned(MessageCoinSigned { amount, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate { amount, .. }) => {
                *non_retryable_balances.entry(AssetId::BASE).or_default() += amount;
            }
            // Sum data messages
            Input::MessageDataSigned(MessageDataSigned { amount, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { amount, .. }) => {
                retryable_balance += *amount;
            }
            Input::Contract(_) => {}
        }
    }

    // Deduct fee from base asset
    let fee = TransactionFee::checked_from_tx(params, transaction).ok_or(CheckError::ArithmeticOverflow)?;

    let base_asset_balance = non_retryable_balances.entry(AssetId::BASE).or_default();

    *base_asset_balance = fee
        .checked_deduct_total(*base_asset_balance)
        .ok_or(CheckError::InsufficientFeeAmount {
            expected: fee.total(),
            provided: *base_asset_balance,
        })?;

    // reduce free balances by coin outputs
    for (asset_id, amount) in transaction.outputs().iter().filter_map(|output| match output {
        Output::Coin { asset_id, amount, .. } => Some((asset_id, amount)),
        _ => None,
    }) {
        let balance = non_retryable_balances
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
        non_retryable_balances,
        retryable_balance,
        fee,
    })
}

pub(crate) struct AvailableBalances {
    pub(crate) non_retryable_balances: BTreeMap<AssetId, Word>,
    pub(crate) retryable_balance: Word,
    pub(crate) fee: TransactionFee,
}

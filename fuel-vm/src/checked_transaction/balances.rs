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
    let mut sum_inputs = BTreeMap::<AssetId, Word>::new();
    // The sum of [`AssetId::Base`] from metadata messages.
    let mut sum_data_messages: Word = 0;

    // Add up all the inputs for each asset ID
    for input in transaction.inputs().iter() {
        match input {
            // Sum coin inputs
            Input::CoinPredicate(CoinPredicate { asset_id, amount, .. })
            | Input::CoinSigned(CoinSigned { asset_id, amount, .. }) => {
                *sum_inputs.entry(*asset_id).or_default() += amount;
            }
            // Sum deposit inputs
            Input::MessageCoinSigned(MessageCoinSigned { amount, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate { amount, .. }) => {
                *sum_inputs.entry(AssetId::BASE).or_default() += amount;
            }
            // Sum data messages
            Input::MessageDataSigned(MessageDataSigned { amount, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { amount, .. }) => {
                sum_data_messages += *amount;
            }
            Input::Contract(_) => {}
        }
    }

    // Deduct fee from base asset
    let fee = TransactionFee::checked_from_tx(params, transaction).ok_or(CheckError::ArithmeticOverflow)?;

    let base_asset_balance = sum_inputs.entry(AssetId::BASE).or_default();

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
        let balance = sum_inputs
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
        sum_inputs,
        sum_data_messages,
        fee,
    })
}

pub(crate) struct AvailableBalances {
    pub(crate) sum_inputs: BTreeMap<AssetId, Word>,
    pub(crate) sum_data_messages: Word,
    pub(crate) fee: TransactionFee,
}

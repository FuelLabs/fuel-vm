use fuel_tx::{
    field,
    input::{
        coin::{CoinPredicate, CoinSigned},
        message::{DepositCoinPredicate, DepositCoinSigned, MetadataPredicate, MetadataSigned},
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
    let mut balances = BTreeMap::<AssetId, Word>::new();

    // Add up all the inputs for each asset ID
    for (asset_id, amount) in transaction.inputs().iter().filter_map(|input| match input {
        // Sum coin inputs
        Input::CoinPredicate(CoinPredicate { asset_id, amount, .. })
        | Input::CoinSigned(CoinSigned { asset_id, amount, .. }) => Some((*asset_id, amount)),
        // Sum message inputs
        Input::DepositCoinSigned(DepositCoinSigned { amount, .. })
        | Input::DepositCoinPredicate(DepositCoinPredicate { amount, .. })
        | Input::MetadataSigned(MetadataSigned { amount, .. })
        | Input::MetadataPredicate(MetadataPredicate { amount, .. }) => Some((AssetId::BASE, amount)),
        Input::Contract(_) => None,
    }) {
        *balances.entry(asset_id).or_default() += amount;
    }

    // Deduct fee from base asset
    let fee = TransactionFee::checked_from_tx(params, transaction).ok_or(CheckError::ArithmeticOverflow)?;

    let base_asset_balance = balances.entry(AssetId::BASE).or_default();

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
    pub(crate) initial_free_balances: BTreeMap<AssetId, Word>,
    pub(crate) fee: TransactionFee,
}

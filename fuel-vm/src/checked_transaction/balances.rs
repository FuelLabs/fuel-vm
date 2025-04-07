use fuel_tx::{
    field,
    input::{
        coin::{
            CoinPredicate,
            CoinSigned,
            DataCoinPredicate,
        },
        message::{
            MessageCoinPredicate,
            MessageCoinSigned,
            MessageDataPredicate,
            MessageDataSigned,
        },
    },
    Chargeable,
    Input,
    Output,
    ValidityError,
};
use fuel_types::{
    AssetId,
    Word,
};

use alloc::collections::BTreeMap;
use fuel_tx::{
    input::{
        coin::{
            DataCoinSigned,
            UnverifiedCoin,
            UnverifiedDataCoin,
        },
        ReadOnly,
    },
    policies::PolicyType,
};

pub(crate) fn initial_free_balances<T>(
    tx: &T,
    base_asset_id: &AssetId,
) -> Result<AvailableBalances, ValidityError>
where
    T: Chargeable + field::Inputs + field::Outputs,
{
    let (mut non_retryable_balances, retryable_balance) =
        add_up_input_balances(tx, base_asset_id).ok_or(ValidityError::BalanceOverflow)?;

    let max_fee = tx
        .policies()
        .get(PolicyType::MaxFee)
        .ok_or(ValidityError::TransactionMaxFeeNotSet)?;
    deduct_max_fee_from_base_asset(&mut non_retryable_balances, base_asset_id, max_fee)?;

    reduce_free_balances_by_coin_outputs(&mut non_retryable_balances, tx)?;

    Ok(AvailableBalances {
        non_retryable_balances,
        retryable_balance,
    })
}

/// Returns None if any of the balances would overflow.
fn add_up_input_balances<T: field::Inputs>(
    transaction: &T,
    base_asset_id: &AssetId,
) -> Option<(BTreeMap<AssetId, Word>, Word)> {
    let mut non_retryable_balances = BTreeMap::<AssetId, Word>::new();
    // The sum of [`AssetId::Base`] from metadata messages.
    let mut retryable_balance: Word = 0;

    // Add up all the inputs for each asset ID
    for input in transaction.inputs().iter() {
        match input {
            // Sum coin inputs
            Input::CoinPredicate(CoinPredicate {
                asset_id, amount, ..
            })
            | Input::DataCoinPredicate(DataCoinPredicate {
                asset_id, amount, ..
            })
            | Input::CoinSigned(CoinSigned {
                asset_id, amount, ..
            }) => {
                let balance = non_retryable_balances.entry(*asset_id).or_default();
                *balance = (*balance).checked_add(*amount)?;
            }
            Input::DataCoinSigned(DataCoinSigned {
                asset_id, amount, ..
            }) => {
                let balance = non_retryable_balances.entry(*asset_id).or_default();
                *balance = (*balance).checked_add(*amount)?;
            }
            Input::ReadOnly(inner) => match inner {
                ReadOnly::VerifiedCoin(CoinPredicate {
                    asset_id, amount, ..
                })
                | ReadOnly::VerifiedDataCoin(DataCoinPredicate {
                    asset_id,
                    amount,
                    ..
                })
                | ReadOnly::UnverifiedCoin(UnverifiedCoin {
                    asset_id, amount, ..
                })
                | ReadOnly::UnverifiedDataCoin(UnverifiedDataCoin {
                    asset_id,
                    amount,
                    ..
                }) => {
                    let balance = non_retryable_balances.entry(*asset_id).or_default();
                    *balance = (*balance).checked_add(*amount)?;
                }
            },
            // Sum message coin inputs
            Input::MessageCoinSigned(MessageCoinSigned { amount, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate { amount, .. }) => {
                let balance = non_retryable_balances.entry(*base_asset_id).or_default();
                *balance = (*balance).checked_add(*amount)?;
            }
            // Sum data messages
            Input::MessageDataSigned(MessageDataSigned { amount, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { amount, .. }) => {
                retryable_balance = retryable_balance.checked_add(*amount)?;
            }
            Input::Contract(_) => {}
        }
    }

    Some((non_retryable_balances, retryable_balance))
}

fn deduct_max_fee_from_base_asset(
    non_retryable_balances: &mut BTreeMap<AssetId, Word>,
    base_asset_id: &AssetId,
    max_fee: Word,
) -> Result<(), ValidityError> {
    let base_asset_balance = non_retryable_balances.entry(*base_asset_id).or_default();
    *base_asset_balance = base_asset_balance.checked_sub(max_fee).ok_or(
        ValidityError::InsufficientFeeAmount {
            expected: max_fee,
            provided: *base_asset_balance,
        },
    )?;

    Ok(())
}

fn reduce_free_balances_by_coin_outputs(
    non_retryable_balances: &mut BTreeMap<AssetId, Word>,
    transaction: &impl field::Outputs,
) -> Result<(), ValidityError> {
    // reduce free balances by coin outputs
    for (asset_id, amount) in transaction
        .outputs()
        .iter()
        .filter_map(Output::coin_balance)
    {
        let balance = non_retryable_balances.get_mut(&asset_id).ok_or(
            ValidityError::TransactionOutputCoinAssetIdNotFound(asset_id),
        )?;
        *balance = balance.checked_sub(amount).ok_or(
            ValidityError::InsufficientInputAmount {
                asset: asset_id,
                expected: amount,
                provided: *balance,
            },
        )?;
    }

    Ok(())
}

pub(crate) struct AvailableBalances {
    pub(crate) non_retryable_balances: BTreeMap<AssetId, Word>,
    pub(crate) retryable_balance: Word,
}

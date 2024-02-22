use fuel_tx::{
    field,
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
    Chargeable,
    FeeParameters,
    GasCosts,
    Input,
    Output,
    TransactionFee,
    ValidityError,
};
use fuel_types::{
    AssetId,
    Word,
};

use alloc::collections::BTreeMap;

pub(crate) fn initial_free_balances<T>(
    transaction: &T,
    gas_costs: &GasCosts,
    params: &FeeParameters,
    base_asset_id: &AssetId,
    gas_price: u64,
) -> Result<AvailableBalances, ValidityError>
where
    T: Chargeable + field::Inputs + field::Outputs,
{
    let (mut non_retryable_balances, retryable_balance) =
        add_up_input_balances(transaction, base_asset_id);

    let fee = TransactionFee::checked_from_tx(gas_costs, params, transaction, gas_price)
        .ok_or(ValidityError::BalanceOverflow)?;

    deduct_fee_from_base_asset(&mut non_retryable_balances, base_asset_id, &fee)?;

    reduce_free_balances_by_coin_outputs(&mut non_retryable_balances, transaction)?;

    Ok(AvailableBalances {
        non_retryable_balances,
        retryable_balance,
        fee,
    })
}

fn add_up_input_balances<T: field::Inputs>(
    transaction: &T,
    base_asset_id: &AssetId,
) -> (BTreeMap<AssetId, Word>, Word) {
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
            | Input::CoinSigned(CoinSigned {
                asset_id, amount, ..
            }) => {
                *non_retryable_balances.entry(*asset_id).or_default() += amount;
            }
            // Sum message coin inputs
            Input::MessageCoinSigned(MessageCoinSigned { amount, .. })
            | Input::MessageCoinPredicate(MessageCoinPredicate { amount, .. }) => {
                *non_retryable_balances.entry(*base_asset_id).or_default() += amount;
            }
            // Sum data messages
            Input::MessageDataSigned(MessageDataSigned { amount, .. })
            | Input::MessageDataPredicate(MessageDataPredicate { amount, .. }) => {
                retryable_balance += *amount;
            }
            Input::Contract(_) => {}
        }
    }

    (non_retryable_balances, retryable_balance)
}

fn deduct_fee_from_base_asset(
    non_retryable_balances: &mut BTreeMap<AssetId, Word>,
    base_asset_id: &AssetId,
    fee: &TransactionFee,
) -> Result<(), ValidityError> {
    let base_asset_balance = non_retryable_balances.entry(*base_asset_id).or_default();

    *base_asset_balance = fee.checked_deduct_total(*base_asset_balance).ok_or(
        ValidityError::InsufficientFeeAmount {
            expected: fee.max_fee(),
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
    for (asset_id, amount) in
        transaction
            .outputs()
            .iter()
            .filter_map(|output| match output {
                Output::Coin {
                    asset_id, amount, ..
                } => Some((asset_id, amount)),
                _ => None,
            })
    {
        let balance = non_retryable_balances.get_mut(asset_id).ok_or(
            ValidityError::TransactionOutputCoinAssetIdNotFound(*asset_id),
        )?;
        *balance = balance.checked_sub(*amount).ok_or(
            ValidityError::InsufficientInputAmount {
                asset: *asset_id,
                expected: *amount,
                provided: *balance,
            },
        )?;
    }

    Ok(())
}

pub(crate) struct AvailableBalances {
    pub(crate) non_retryable_balances: BTreeMap<AssetId, Word>,
    pub(crate) retryable_balance: Word,
    pub(crate) fee: TransactionFee,
}

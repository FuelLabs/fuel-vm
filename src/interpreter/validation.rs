use crate::error::InterpreterError;
use crate::interpreter::Interpreter;
use fuel_asm::PanicReason;
use fuel_tx::{Input, Output, Transaction};
use fuel_types::{AssetId, Word};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedTransaction {
    /// the validated transaction
    pub(crate) tx: Transaction,
    /// the initial free balances of the transaction
    pub(crate) balances: HashMap<AssetId, Word>,
}

impl AsRef<Transaction> for CheckedTransaction {
    fn as_ref(&self) -> &Transaction {
        &self.tx
    }
}

impl<T> Interpreter<T> {
    /// Validate a transaction for later VM execution. The checked transaction wrapper will
    /// allow the VM to trust the transaction as valid and execute it at some point later on in
    /// the future. Note - this doesn't mean the transaction has been completely validated,
    /// predicates must be externally checked as well.
    pub fn check_transaction(
        mut transaction: Transaction,
        height: Word,
    ) -> Result<CheckedTransaction, InterpreterError> {
        // verify transaction - transactions don't expire, once a tx passes the maturity
        // requirements the height doesn't need to be validated again in the future.
        transaction.validate_without_signature(height)?;
        transaction.precompute_metadata();

        // verify & cache initial balances
        let balances = Self::initial_free_balances(&transaction)?;

        Ok(CheckedTransaction {
            tx: transaction,
            balances,
        })
    }

    // compute the initial free balances for each asset type
    pub(crate) fn initial_free_balances(tx: &Transaction) -> Result<HashMap<AssetId, Word>, InterpreterError> {
        let mut balances = HashMap::<AssetId, Word>::new();

        // Add up all the inputs for each asset ID
        for (asset_id, amount) in tx.inputs().iter().filter_map(|input| match input {
            Input::Coin { asset_id, amount, .. } => Some((asset_id, amount)),
            _ => None,
        }) {
            *balances.entry(*asset_id).or_default() += amount;
        }

        // Reduce by unavailable balances
        let base_asset = AssetId::default();
        if let Some(base_asset_balance) = balances.get_mut(&base_asset) {
            // remove byte costs from base asset spendable balance
            let byte_balance = (tx.metered_bytes_size() as Word) * tx.byte_price();
            *base_asset_balance = base_asset_balance
                .checked_sub(byte_balance)
                .ok_or(InterpreterError::Panic(PanicReason::NotEnoughBalance))?;
            // remove gas costs from base asset spendable balance
            *base_asset_balance = base_asset_balance
                .checked_sub(tx.gas_limit() * tx.gas_price())
                .ok_or(InterpreterError::Panic(PanicReason::NotEnoughBalance))?;
        }

        // reduce free balances by coin and withdrawal outputs
        for (asset_id, amount) in tx.outputs().iter().filter_map(|output| match output {
            Output::Coin { asset_id, amount, .. } => Some((asset_id, amount)),
            Output::Withdrawal { asset_id, amount, .. } => Some((asset_id, amount)),
            _ => None,
        }) {
            let balance = balances.get_mut(asset_id).unwrap();
            *balance = balance
                .checked_sub(*amount)
                .ok_or(InterpreterError::Panic(PanicReason::NotEnoughBalance))?;
        }

        Ok(balances)
    }
}

//! Bundle execution of transaction

use std::collections::{HashMap, HashSet};

use fuel_tx::{Transaction, UtxoId, Output, ValidationError};

use crate::{
    prelude::Transactor,
    storage::InterpreterStorage,
};

mod metadata;
mod substate;

pub use substate::{Metadata, SubState};

/// Bundle transaction execution into one module.
pub struct BundleTransactor<'a, STORAGE> {
    /// executed and commited transactions.
    /// Maybe even had TxId in vec for order and HashMap<TxId,Tx> for fast fetching.
    transactions: Vec<Transaction>,
    /// outputs that are spend or newly created by one of previous transactions in bundle.
    /// Can probably be done outside of VM but for not leave it here.
    outputs: HashMap<UtxoId, UtxoStatus>,
    /// pending used outputs
    pending_outputs: HashMap<UtxoId,UtxoStatus>,
    /// Executor
    transactor: Transactor<'a, SubState<STORAGE>>,
}


pub enum UtxoStatus {
    Spend,
    Unspend,
    FromDB,
}


impl<STORAGE> BundleTransactor<'_, STORAGE> {
    pub fn new(state: STORAGE, metadata: Metadata) -> Self {
        Self {
            transactions: Vec::new(),
            outputs: HashMap::new(),
            pending_outputs: HashMap::new(),
            transactor: Transactor::new(SubState::new(state, metadata)),
        }
    }

    pub fn storage(&self) {}


    fn check_dependent_outputs(&mut self, mut tx: Transaction, pre_checked_outputs: HashMap<UtxoId,bool>) -> Result<Transaction,ValidationError> {
        // iterate over all inputs and check ones that are not prechecked before execution
        for input in tx.inputs() {
            let utxo_id = input.utxo_id();
            // check if we precheked this output. This includes db.
            if let Some(is_db) = pre_checked_outputs.get(utxo_id) {
                self.pending_outputs.insert(utxo_id.clone(), if *is_db {UtxoStatus::FromDB} else {UtxoStatus::Spend});
            }



        }

        Ok(tx)
    }
}

impl<STORAGE> BundleTransactor<'_, STORAGE>
where
    STORAGE: InterpreterStorage,
{
    /// transaction that we want to include.
    /// Idea of db_outputs is to prefetch data and check them before tx is received inside BundleTransactor
    pub fn transact(&mut self, tx: Transaction, pre_checked_outputs: HashMap<UtxoId,bool>) -> &mut Self {
        // TODO clean state
        self.transactor.transact(tx);
        self
    }

    /// Commit last executed transaction
    pub fn commit(&mut self) {

    }

    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    pub fn outputs(&self) -> &HashMap<UtxoId, UtxoStatus> {
        &self.outputs
    }

    /// cleanup the state and return list of executed transactions and used outputs
    pub fn finalize(&mut self) {}
}

impl<'a,STORAGE> AsRef<Transactor<'a,SubState<STORAGE>>> for BundleTransactor<'a,STORAGE> {
    fn as_ref(&self) -> &Transactor<'a,SubState<STORAGE>> {
        &self.transactor
    }
}

impl<'a,STORAGE> AsMut<Transactor<'a,SubState<STORAGE>>> for BundleTransactor<'a,STORAGE> {
    fn as_mut(&mut self) -> &mut Transactor<'a,SubState<STORAGE>> {
        &mut self.transactor
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn initial_test() {
        //let bundler = BundleTransactor::
    }
}

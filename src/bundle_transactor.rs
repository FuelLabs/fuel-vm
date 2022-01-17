//! Bundle execution of transaction

use hashbrown::HashMap;

use fuel_tx::{Receipt, Transaction, TxId, UtxoId};

use crate::{prelude::Transactor, storage::InterpreterStorage};

mod metadata;
mod substorage;

pub use substorage::{Metadata, SubStorage};

/// Bundle transaction execution into one module.
pub struct BundleTransactor<'a, STORAGE> {
    /// executed and commited transactions.
    /// Maybe even had TxId in vec for order and HashMap<TxId,Tx> for fast fetching.
    transactions: Vec<(Transaction, Vec<Receipt>)>,
    /// tx_id to index;
    transction_order: HashMap<TxId, usize>,
    /// outputs that are spend or newly created by one of previous transactions in bundle.
    /// Can probably be done outside of VM but for not leave it here.
    outputs: HashMap<UtxoId, UtxoStatus>,
    /// pending used outputs
    pending_outputs: HashMap<UtxoId, UtxoStatus>,
    /// Executor
    transactor: Transactor<'a, SubStorage<STORAGE>>,
}

pub enum UtxoStatus {
    Spend,
    Unspend,
    FromDB,
}

impl<STORAGE> BundleTransactor<'_, STORAGE> {
    /// Create bundler.
    pub fn new(storage: STORAGE, metadata: Metadata) -> Self {
        Self {
            transactions: Vec::new(),
            outputs: HashMap::new(),
            transction_order: HashMap::new(),
            pending_outputs: HashMap::new(),
            transactor: Transactor::new(SubStorage::new(storage, metadata)),
        }
    }

    /// Get reference to internal storage.
    pub fn storage(&self) -> &SubStorage<STORAGE> {
        self.transactor.as_ref()
    }

    /// Iterate over all inputs and check ones that are not pre_checked. Add all outputs to pending_outputs.
    fn check_dependent_outputs(
        &mut self,
        tx: &Transaction,
        pre_checked_inputs: &HashMap<UtxoId, bool>,
    ) -> Result<(), TransactorError> {
        // iterate over all inputs and check ones that are not prechecked before execution
        for input in tx.inputs() {
            let utxo_id = input.utxo_id();
            // check if we precheked this output. This includes db.
            if let Some(is_db) = pre_checked_inputs.get(utxo_id) {
                self.pending_outputs.insert(
                    utxo_id.clone(),
                    if *is_db { UtxoStatus::FromDB } else { UtxoStatus::Spend },
                );
            }
            // TODO
            self.pending_outputs.insert(utxo_id.clone(), UtxoStatus::Spend);
        }

        //TODO iterate over outputs and add them
        for (output_index, _) in tx.outputs().iter().enumerate() {
            // Safety: it is okay to cast to u8 bcs output number is already checked.
            let utxo_id = UtxoId::new(tx.id(), output_index as u8);
            self.pending_outputs.insert(utxo_id, UtxoStatus::Unspend);
        }

        Ok(())
    }
}

/// Bundler errors.
pub enum TransactorError {
    /// None
    None,
    /// Some
    Some,
    /// Hello
    HelloError,
}

impl<STORAGE> BundleTransactor<'_, STORAGE>
where
    STORAGE: InterpreterStorage,
{
    /// transaction that we want to include.
    /// Idea of pre_checked_output is to do precalculation on inputs/outputs before it comes to transactor.and
    /// in transactor check only variable outputs that we dont know value beforehand.
    /// TODO specify what is expected and what checks we dont need to do
    pub fn transact(
        &mut self,
        tx: Transaction,
        pre_checked_inputs: &HashMap<UtxoId, bool>,
    ) -> Result<&mut Self, TransactorError> {
        // TODO clean state
        self.check_dependent_outputs(&tx, pre_checked_inputs)?;
        self.transactor.transact(tx);
        Ok(self)
    }

    /// Commit last executed transaction and do cleanup on state
    pub fn commit(&mut self) -> &mut Self {
        if let Ok(transition) = self.transactor.result() {
            let outputs = self.pending_outputs.drain();
            self.outputs.extend(outputs.into_iter());
            // commit transaction transition to bundler.
            let index = self.transactions.len();
            self.transactions
                .push((transition.tx().clone(), transition.receipts().to_vec()));
            self.transction_order.insert(transition.tx().id(), index);
            self.transactor.as_mut().commit_pending();
            self.transactor.clear();
        } else {
            self.transactor.as_mut().reject_pending();
        }
        self
    }

    pub fn transactions(&self) -> &[(Transaction, Vec<Receipt>)] {
        &self.transactions
    }

    pub fn outputs(&self) -> &HashMap<UtxoId, UtxoStatus> {
        &self.outputs
    }

    /// cleanup the state and return list of executed transactions and used outputs
    pub fn finalize(&mut self) {}
}

// For transactor
impl<'a, STORAGE> AsRef<Transactor<'a, SubStorage<STORAGE>>> for BundleTransactor<'a, STORAGE> {
    fn as_ref(&self) -> &Transactor<'a, SubStorage<STORAGE>> {
        &self.transactor
    }
}

// For storage
impl<'a, STORAGE> AsRef<SubStorage<STORAGE>> for BundleTransactor<'a, STORAGE> {
    fn as_ref(&self) -> &SubStorage<STORAGE> {
        self.transactor.as_ref()
    }
}

// Mut for transactor
impl<'a, STORAGE> AsMut<Transactor<'a, SubStorage<STORAGE>>> for BundleTransactor<'a, STORAGE> {
    fn as_mut(&mut self) -> &mut Transactor<'a, SubStorage<STORAGE>> {
        &mut self.transactor
    }
}

// Mut for storage
impl<'a, STORAGE> AsMut<SubStorage<STORAGE>> for BundleTransactor<'a, STORAGE> {
    fn as_mut(&mut self) -> &mut SubStorage<STORAGE> {
        self.transactor.as_mut()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::memory_client::MemoryStorage;

    #[test]
    fn initial_test() {
        let metadata = Metadata::default();
        let storage = MemoryStorage::new(metadata.block_height(), *metadata.coinbase());
        let bundler = BundleTransactor::new(storage, metadata);
        //bundler.transact()
    }
}

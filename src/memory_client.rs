//! In-memory client implementation

use crate::backtrace::Backtrace;
use crate::state::StateTransitionRef;
use crate::storage::MemoryStorage;
use crate::transactor::Transactor;

use fuel_tx::{ConsensusParameters, Receipt, Transaction};

#[derive(Debug, Default)]
/// Client implementation with in-memory storage backend.
pub struct MemoryClient<'a> {
    transactor: Transactor<'a, MemoryStorage>,
}

impl<'a> AsRef<MemoryStorage> for MemoryClient<'a> {
    fn as_ref(&self) -> &MemoryStorage {
        self.transactor.as_ref()
    }
}

impl<'a> AsMut<MemoryStorage> for MemoryClient<'a> {
    fn as_mut(&mut self) -> &mut MemoryStorage {
        self.transactor.as_mut()
    }
}

impl<'a> MemoryClient<'a> {
    /// Create a new instance of the memory client out of a provided storage.
    pub fn new(storage: MemoryStorage, params: ConsensusParameters) -> Self {
        Self {
            transactor: Transactor::new(storage, params),
        }
    }

    /// Create a new instance of the memory client out of a provided storage.
    pub fn from_txtor(transactor: Transactor<'a, MemoryStorage>) -> Self {
        Self { transactor }
    }

    /// If a transaction was executed and produced a VM panic, returns the
    /// backtrace; return `None` otherwise.
    pub fn backtrace(&self) -> Option<Backtrace> {
        self.transactor.backtrace()
    }

    /// If a transaction was successfully executed, returns the produced
    /// receipts; return `None` otherwise.
    pub fn receipts(&self) -> Option<&[Receipt]> {
        self.transactor.receipts()
    }

    /// State transition representation after the execution of a transaction.
    pub const fn state_transition(&self) -> Option<&StateTransitionRef<'_>> {
        self.transactor.state_transition()
    }

    /// Execute a transaction.
    ///
    /// Since the memory storage is `Infallible`, associatively, the memory
    /// client should also be.
    pub fn transact(&mut self, tx: Transaction) -> &[Receipt] {
        self.transactor.transact(tx);

        // TODO `Transactor::result` should accept error as generic so compile-time
        // constraints can be applied.
        //
        // In this case, we should expect `Infallible` error.
        if let Ok(state) = self.transactor.result() {
            if state.should_revert() {
                self.transactor.as_mut().revert();
            } else {
                self.transactor.as_mut().commit();
            }
        } else {
            // if vm failed to execute, revert storage just in case
            self.transactor.as_mut().revert();
        }

        self.transactor.receipts().unwrap_or_default()
    }

    /// Persist the changes caused by [`Self::transact`].
    pub fn persist(&mut self) {
        self.as_mut().persist();
    }

    /// Consensus parameters
    pub const fn params(&self) -> &ConsensusParameters {
        self.transactor.params()
    }

    /// Tx memory offset
    pub const fn tx_offset(&self) -> usize {
        self.transactor.tx_offset()
    }
}

impl<'a> From<MemoryStorage> for MemoryClient<'a> {
    fn from(s: MemoryStorage) -> Self {
        Self::new(s, Default::default())
    }
}

impl<'a> From<MemoryClient<'a>> for Transactor<'a, MemoryStorage> {
    fn from(client: MemoryClient<'a>) -> Self {
        client.transactor
    }
}

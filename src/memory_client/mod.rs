//! In-memory client implementation

use crate::backtrace::Backtrace;
use crate::transactor::Transactor;

use fuel_tx::{Receipt, Transaction};

mod storage;

pub use storage::MemoryStorage;

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
    pub fn new(storage: MemoryStorage) -> Self {
        Self {
            transactor: Transactor::new(storage),
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
        let state = self
            .transactor
            .result()
            .expect("MemoryStorage implements `Infallible` as storage error. This means panic should be unreachable.");

        if state.should_revert() {
            self.transactor.as_mut().revert();
        } else {
            self.transactor.as_mut().commit();
        }

        self.transactor
            .receipts()
            .expect("The transaction was provided to the transactor.")
    }

    /// Persist the changes caused by [`Self::transact`].
    pub fn persist(&mut self) {
        self.as_mut().persist();
    }
}

impl<'a> From<MemoryStorage> for MemoryClient<'a> {
    fn from(s: MemoryStorage) -> Self {
        Self::new(s)
    }
}

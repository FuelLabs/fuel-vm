use crate::backtrace::Backtrace;
use crate::error::InterpreterError;
use crate::transactor::Transactor;

use fuel_tx::{Receipt, Transaction};

mod storage;

pub use storage::MemoryStorage;

#[derive(Debug, Default, Clone)]
pub struct MemoryClient<'a> {
    transactor: Transactor<'a, MemoryStorage>,
}

impl<'a> From<MemoryStorage> for MemoryClient<'a> {
    fn from(s: MemoryStorage) -> Self {
        Self::new(s)
    }
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
    pub fn new(storage: MemoryStorage) -> Self {
        Self {
            transactor: Transactor::new(storage),
        }
    }

    pub fn backtrace(&self) -> Option<Backtrace> {
        self.transactor.backtrace()
    }

    pub fn receipts(&self) -> Option<&[Receipt]> {
        self.transactor.receipts()
    }

    pub fn transact(&mut self, tx: Transaction) -> Result<&[Receipt], InterpreterError> {
        self.transactor.transact(tx);

        match self.transactor.result() {
            Ok(state) if state.should_revert() => self.transactor.as_mut().revert(),

            Ok(_) => self.transactor.as_mut().commit(),

            Err(e) => {
                self.transactor.as_mut().rollback();

                return Err(e);
            }
        };

        self.transactor
            .receipts()
            .ok_or(InterpreterError::NoTransactionInitialized)
    }

    pub fn persist(&mut self) {
        self.as_mut().persist();
    }
}

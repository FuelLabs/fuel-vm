use crate::error::InterpreterError;
use crate::interpreter::Interpreter;

use fuel_tx::{Receipt, Transaction};

mod storage;

pub use storage::MemoryStorage;

#[derive(Debug, Default, Clone)]
pub struct MemoryClient {
    storage: MemoryStorage,
}

impl From<MemoryStorage> for MemoryClient {
    fn from(s: MemoryStorage) -> Self {
        Self::new(s)
    }
}

impl AsRef<MemoryStorage> for MemoryClient {
    fn as_ref(&self) -> &MemoryStorage {
        &self.storage
    }
}

impl AsMut<MemoryStorage> for MemoryClient {
    fn as_mut(&mut self) -> &mut MemoryStorage {
        &mut self.storage
    }
}

impl MemoryClient {
    pub const fn new(storage: MemoryStorage) -> Self {
        Self { storage }
    }

    pub fn transition<I: IntoIterator<Item = Transaction>>(
        &mut self,
        txs: I,
    ) -> Result<Vec<Receipt>, InterpreterError> {
        let mut interpreter = Interpreter::with_storage(&mut self.storage);

        // A transaction that is considered valid will be reverted if the runtime
        // returns an error.
        //
        // The exception is `InterpreterError::ValidationError` since this means the
        // whole set of transactions must also fail.
        //
        // All the other variants of `InterpreterError` are supressed in this function
        // and they will produce isolated `RVRT` operations.
        let receipts = txs.into_iter().try_fold(vec![], |mut receipts, tx| {
            match interpreter.transact(tx) {
                Ok(state) => {
                    receipts.extend(state.receipts());

                    if !state.receipts().iter().any(|r| matches!(r, Receipt::Revert { .. })) {
                        interpreter.as_mut().commit();
                    } else {
                        interpreter.as_mut().revert();
                    }

                    Ok(receipts)
                }

                Err(InterpreterError::ValidationError(e)) => {
                    interpreter.as_mut().rollback();

                    Err(InterpreterError::ValidationError(e))
                }

                // TODO VM is to return a `RVRT` receipt on runtime error. This way, the return of
                // `transact` should be `Err` only if `InterpreterError::ValidationError` happens
                Err(_) => {
                    interpreter.as_mut().revert();

                    Ok(receipts)
                }
            }
        })?;

        self.storage.persist();

        Ok(receipts)
    }
}

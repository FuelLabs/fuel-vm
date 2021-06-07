use super::{ExecuteError, Interpreter};
use crate::data::InterpreterStorage;

use fuel_tx::bytes::Deserializable;
use fuel_tx::Transaction;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub fn execute_tx_bytes(storage: S, bytes: &[u8]) -> Result<Self, ExecuteError> {
        let tx = Transaction::from_bytes(bytes)?;

        Self::execute_tx(storage, tx)
    }

    pub fn execute_tx(storage: S, tx: Transaction) -> Result<Self, ExecuteError> {
        let mut vm = Interpreter::with_storage(storage);

        vm.init(tx)?;
        vm.run()?;

        Ok(vm)
    }
}

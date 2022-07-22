use super::Interpreter;
use crate::consts::*;
use crate::context::Context;
use crate::error::InterpreterError;
use crate::interpreter::RuntimeBalances;
use crate::storage::InterpreterStorage;

use fuel_tx::CheckedTransaction;
use fuel_types::bytes::SizedBytes;
use fuel_types::Word;

use std::io;

impl<S> Interpreter<S> {
    /// Initialize the VM with a given transaction
    pub fn init(&mut self, tx: CheckedTransaction) -> Result<(), InterpreterError> {
        self.tx = tx;

        self.frames.clear();
        self.receipts.clear();

        // Optimized for memset
        self.registers.iter_mut().for_each(|r| *r = 0);

        self.registers[REG_ONE] = 1;
        self.registers[REG_SSP] = 0;

        // Set heap area
        self.registers[REG_HP] = VM_MAX_RAM - 1;

        self.push_stack(self.transaction().id().as_ref())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        RuntimeBalances::from(&self.tx).to_vm(self);

        let tx_size = self.transaction().serialized_size() as Word;

        self.registers[REG_GGAS] = self.transaction().gas_limit();
        self.registers[REG_CGAS] = self.transaction().gas_limit();

        self.push_stack(&tx_size.to_be_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let tx = self.tx.tx_bytes();

        self.push_stack(tx.as_slice())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.registers[REG_SP] = self.registers[REG_SSP];

        Ok(())
    }

    /// Initialize the VM for a predicate context
    pub fn init_predicate(&mut self, tx: CheckedTransaction) -> bool {
        self.context = Context::Predicate {
            program: Default::default(),
        };

        self.init(tx).is_ok()
    }
}

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    /// Initialize the VM with a given transaction, backed by a storage provider that allows
    /// execution of contract opcodes.
    ///
    /// For predicate verification, check [`Self::init`]
    pub fn init_with_storage(&mut self, tx: CheckedTransaction) -> Result<(), InterpreterError> {
        self.context = Context::Script;

        self.init(tx)
    }
}

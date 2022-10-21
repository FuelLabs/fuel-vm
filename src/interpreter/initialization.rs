use super::{CheckedMetadata, ExecutableTransaction, InitialBalances, Interpreter, RuntimeBalances};
use crate::consts::*;
use crate::context::Context;
use crate::error::InterpreterError;
use crate::storage::InterpreterStorage;

use fuel_tx::{Checked, IntoChecked, Stage, Transaction};
use fuel_types::bytes::{SerializableVec, SizedBytes};
use fuel_types::Word;

use std::io;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    /// Initialize the VM with a given transaction
    fn _init(&mut self, tx: Tx, initial_balances: InitialBalances) -> Result<(), InterpreterError> {
        // TODO: Remove cloning of the transaction and convert it into the `Transaction`.
        //  VM can work directly with inner types. It is only added to make the
        //  code as it was before the refactoring.
        self.tx = tx.clone();
        let mut transaction: Transaction = tx.into();

        self.initial_balances = initial_balances.clone();

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

        RuntimeBalances::from(initial_balances).to_vm(self);

        let tx_size = transaction.serialized_size() as Word;

        self.registers[REG_GGAS] = self.transaction().limit();
        self.registers[REG_CGAS] = self.transaction().limit();

        self.push_stack(&tx_size.to_be_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let tx_bytes = transaction.to_bytes();

        self.push_stack(tx_bytes.as_slice())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.registers[REG_SP] = self.registers[REG_SSP];

        Ok(())
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
    <Tx as IntoChecked>::Metadata: CheckedMetadata,
{
    /// Initialize the VM for a predicate context
    pub fn init_predicate<St: Stage>(&mut self, checked: Checked<Tx, St>) -> bool {
        self.context = Context::Predicate {
            program: Default::default(),
        };

        let (mut tx, metadata): (Tx, Tx::Metadata) = checked.into();
        tx.prepare_init_predicate();

        self._init(tx, metadata.balances()).is_ok()
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    <Tx as IntoChecked>::Metadata: CheckedMetadata,
{
    /// Initialize the VM with a given transaction, backed by a storage provider that allows
    /// execution of contract opcodes.
    ///
    /// For predicate verification, check [`Self::init_predicate`]
    pub fn init_script<St: Stage>(&mut self, checked: Checked<Tx, St>) -> Result<(), InterpreterError> {
        let block_height = self.storage.block_height().map_err(InterpreterError::from_io)?;

        self.context = Context::Script { block_height };

        let (mut tx, metadata): (Tx, Tx::Metadata) = checked.into();
        tx.prepare_init_script();

        self._init(tx, metadata.balances())
    }
}

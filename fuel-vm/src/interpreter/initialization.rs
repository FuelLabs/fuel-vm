use super::{
    ExecutableTransaction,
    InitialBalances,
    Interpreter,
    RuntimeBalances,
};
use crate::{
    checked_transaction::{
        IntoChecked,
        Ready,
    },
    consts::*,
    context::Context,
    error::InterpreterError,
    prelude::RuntimeError,
    storage::InterpreterStorage,
};
use fuel_asm::RegId;
use fuel_tx::field::ScriptGasLimit;
use fuel_types::Word;

use crate::interpreter::CheckedMetadata;

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
{
    /// Initialize the VM with a given transaction
    fn init_inner(
        &mut self,
        mut tx: Tx,
        initial_balances: InitialBalances,
        runtime_balances: RuntimeBalances,
        gas_limit: Word,
    ) -> Result<(), RuntimeError<S::DataError>> {
        tx.prepare_init_execute();
        self.tx = tx;

        self.initial_balances = initial_balances.clone();

        self.frames.clear();
        self.receipts.clear();

        // Optimized for memset
        self.registers.iter_mut().for_each(|r| *r = 0);

        self.registers[RegId::ONE] = 1;
        self.registers[RegId::SSP] = 0;

        // Set heap area
        self.registers[RegId::HP] = VM_MAX_RAM;

        self.push_stack(self.transaction().id(&self.chain_id()).as_ref())?;

        runtime_balances.to_vm(self);

        let tx_size = self.transaction().size() as Word;
        self.set_gas(gas_limit);

        self.push_stack(&tx_size.to_be_bytes())?;

        let tx_bytes = self.tx.to_bytes();

        self.push_stack(tx_bytes.as_slice())?;

        self.registers[RegId::SP] = self.registers[RegId::SSP];

        Ok(())
    }
}

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
{
    /// Initialize the VM for a predicate context
    pub fn init_predicate(
        &mut self,
        context: Context,
        tx: Tx,
        gas_limit: Word,
    ) -> Result<(), InterpreterError<S::DataError>> {
        self.context = context;
        let initial_balances: InitialBalances = Default::default();
        let runtime_balances = initial_balances.clone().try_into()?;
        Ok(self.init_inner(tx, initial_balances, runtime_balances, gas_limit)?)
    }
}

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
where
    S: InterpreterStorage,
    <S as InterpreterStorage>::DataError: From<S::DataError>,
    Tx: ExecutableTransaction,
    <Tx as IntoChecked>::Metadata: CheckedMetadata,
{
    /// Initialize the VM with a given transaction, backed by a storage provider that
    /// allows execution of contract opcodes.
    ///
    /// For predicate estimation and verification, check [`Self::init_predicate`]
    pub fn init_script(
        &mut self,
        immutable: Ready<Tx>,
    ) -> Result<(), InterpreterError<S::DataError>> {
        let block_height = self.storage.block_height().map_err(RuntimeError::Storage)?;

        self.context = Context::Script { block_height };

        let (_, tx, metadata, _) = immutable.decompose();

        let gas_limit = tx
            .as_script()
            .map(|script| *script.script_gas_limit())
            .unwrap_or_default();

        let initial_balances = metadata.balances();
        let runtime_balances = initial_balances.try_into()?;
        Ok(self.init_inner(tx, metadata.balances(), runtime_balances, gas_limit)?)
    }
}

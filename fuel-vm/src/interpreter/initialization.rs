use super::{CheckedMetadata, ExecutableTransaction, InitialBalances, Interpreter, RuntimeBalances};
use crate::checked_transaction::{Checked, IntoChecked};
use crate::consts::*;
use crate::context::Context;
use crate::error::{Bug, BugId, InterpreterError};
use crate::storage::InterpreterStorage;

use fuel_asm::RegId;
use fuel_types::Word;

use crate::error::BugVariant::GlobalGasUnderflow;
use std::io;
use crate::estimated_transaction::{Estimated, IntoEstimated};

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    /// Initialize the VM with a given transaction
    fn _init(
        &mut self,
        tx: Tx,
        initial_balances: InitialBalances,
        gas_used_by_predicates: Word,
    ) -> Result<(), InterpreterError> {
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

        self.push_stack(self.transaction().id(&self.params).as_ref())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        RuntimeBalances::try_from(initial_balances)?.to_vm(self);

        let tx_size = self.transaction().serialized_size() as Word;

        let gas = self
            .transaction()
            .limit()
            .checked_sub(gas_used_by_predicates)
            .ok_or_else(|| Bug::new(BugId::ID003, GlobalGasUnderflow))?;

        self.set_remaining_gas(gas);

        self.push_stack(&tx_size.to_be_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let tx_bytes = self.tx.to_bytes();

        self.push_stack(tx_bytes.as_slice())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.registers[RegId::SP] = self.registers[RegId::SSP];

        Ok(())
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
    <Tx as IntoEstimated>::Metadata: CheckedMetadata,
{
    /// Initialize the VM for a predicate context
    pub fn init_predicate_estimation(&mut self, estimated: Estimated<Tx>) -> bool {
        self.context = Context::PredicateEstimation {
            program: Default::default(),
        };

        let (mut tx, metadata): (Tx, Tx::Metadata) = estimated.into();
        tx.prepare_init_predicate();

        self._init(tx, metadata.balances(), 0).is_ok()
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
    <Tx as IntoChecked>::Metadata: CheckedMetadata,
{
    /// Initialize the VM for a predicate context
    pub fn init_predicate(&mut self, checked: Checked<Tx>) -> bool {
        self.context = Context::PredicateVerification {
            program: Default::default(),
        };

        let (mut tx, metadata): (Tx, Tx::Metadata) = checked.into();
        tx.prepare_init_predicate();

        self._init(tx, metadata.balances(), 0).is_ok()
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
    /// For predicate estimation and verification, check [`Self::init_predicate`]
    pub fn init_script(&mut self, checked: Checked<Tx>) -> Result<(), InterpreterError> {
        let block_height = self.storage.block_height().map_err(InterpreterError::from_io)?;

        self.context = Context::Script { block_height };

        let gas_used_by_predicates = checked.metadata().gas_used_by_predicates();
        let (mut tx, metadata): (Tx, Tx::Metadata) = checked.into();
        tx.prepare_init_script();

        self._init(tx, metadata.balances(), gas_used_by_predicates)
    }
}

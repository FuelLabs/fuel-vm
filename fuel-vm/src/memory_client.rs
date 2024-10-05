//! In-memory client implementation

use crate::{
    backtrace::Backtrace,
    checked_transaction::Checked,
    error::InterpreterError,
    interpreter::{
        EcalHandler,
        InterpreterParams,
        Memory,
        NotSupportedEcal,
    },
    state::StateTransitionRef,
    storage::MemoryStorage,
    transactor::Transactor,
};
use core::convert::Infallible;
use fuel_tx::{
    Blob,
    Create,
    FeeParameters,
    GasCosts,
    Receipt,
    Script,
    Upgrade,
    Upload,
};

#[cfg(any(test, feature = "test-helpers"))]
use crate::interpreter::MemoryInstance;

#[derive(Debug)]
/// Client implementation with in-memory storage backend.
pub struct MemoryClient<M, Ecal = NotSupportedEcal> {
    transactor: Transactor<M, MemoryStorage, Script, Ecal>,
}

#[cfg(any(test, feature = "test-helpers"))]
impl Default for MemoryClient<MemoryInstance> {
    fn default() -> Self {
        Self::from_txtor(Transactor::new(
            MemoryInstance::new(),
            MemoryStorage::default(),
            InterpreterParams::default(),
        ))
    }
}

impl<M, Ecal: EcalHandler> AsRef<MemoryStorage> for MemoryClient<M, Ecal> {
    fn as_ref(&self) -> &MemoryStorage {
        self.transactor.as_ref()
    }
}

impl<M, Ecal: EcalHandler> AsMut<MemoryStorage> for MemoryClient<M, Ecal> {
    fn as_mut(&mut self) -> &mut MemoryStorage {
        self.transactor.as_mut()
    }
}

impl<M, Ecal: EcalHandler + Default> MemoryClient<M, Ecal> {
    /// Create a new instance of the memory client out of a provided storage.
    pub fn new(
        memory: M,
        storage: MemoryStorage,
        interpreter_params: InterpreterParams,
    ) -> Self {
        Self {
            transactor: Transactor::new(memory, storage, interpreter_params),
        }
    }
}

impl<M, Ecal: EcalHandler> MemoryClient<M, Ecal> {
    /// Create a new instance of the memory client out of a provided storage.
    pub fn from_txtor(transactor: Transactor<M, MemoryStorage, Script, Ecal>) -> Self {
        Self { transactor }
    }
}

impl<M, Ecal: EcalHandler> MemoryClient<M, Ecal>
where
    M: Memory,
{
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
    pub fn state_transition(&self) -> Option<StateTransitionRef<'_, Script>> {
        self.transactor.state_transition()
    }

    /// Deploys a `Create` transaction.
    pub fn deploy(
        &mut self,
        tx: Checked<Create>,
    ) -> Result<Create, InterpreterError<Infallible>> {
        self.transactor.deploy(tx)
    }

    /// Executes `Upgrade` transaction.
    pub fn upgrade(
        &mut self,
        tx: Checked<Upgrade>,
    ) -> Result<Upgrade, InterpreterError<Infallible>> {
        self.transactor.upgrade(tx)
    }

    /// Executes `Upload` transaction.
    pub fn upload(&mut self, tx: Checked<Upload>) -> Option<Upload> {
        self.transactor.upload(tx).ok()
    }

    /// Executes `Blob` transaction.
    pub fn blob(&mut self, tx: Checked<Blob>) -> Option<Blob> {
        self.transactor.blob(tx).ok()
    }

    /// Execute a transaction.
    ///
    /// Since the memory storage is `Infallible`, associatively, the memory
    /// client should also be.
    pub fn transact(&mut self, tx: Checked<Script>) -> &[Receipt] {
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

    /// Tx memory offset
    pub fn tx_offset(&self) -> usize {
        self.transactor.tx_offset()
    }

    /// Gas costs for opcodes
    pub fn gas_costs(&self) -> &GasCosts {
        self.transactor.gas_costs()
    }

    /// Fee parameters
    pub fn fee_params(&self) -> &FeeParameters {
        self.transactor.fee_params()
    }

    #[cfg(feature = "test-helpers")]
    /// Sets the gas price of the `Interpreter`
    pub fn set_gas_price(&mut self, gas_price: u64) {
        self.transactor.set_gas_price(gas_price);
    }
}

impl<M, Ecal: EcalHandler> From<MemoryClient<M, Ecal>>
    for Transactor<M, MemoryStorage, Script, Ecal>
{
    fn from(client: MemoryClient<M, Ecal>) -> Self {
        client.transactor
    }
}

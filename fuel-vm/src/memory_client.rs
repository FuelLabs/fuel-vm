//! In-memory client implementation

use crate::{
    backtrace::Backtrace,
    checked_transaction::Checked,
    error::InterpreterError,
    interpreter::{
        EcalHandler,
        InterpreterParams,
        NotSupportedEcal,
    },
    state::StateTransitionRef,
    storage::MemoryStorage,
    transactor::Transactor,
};
use core::convert::Infallible;
use fuel_tx::{
    Create,
    FeeParameters,
    GasCosts,
    Receipt,
    Script,
    Upgrade,
    Upload,
};

#[derive(Debug)]
/// Client implementation with in-memory storage backend.
pub struct MemoryClient<Ecal = NotSupportedEcal> {
    transactor: Transactor<MemoryStorage, Script, Ecal>,
}

#[cfg(any(test, feature = "test-helpers"))]
impl Default for MemoryClient {
    fn default() -> Self {
        Self::new(MemoryStorage::default(), InterpreterParams::default())
    }
}

impl<Ecal: EcalHandler> AsRef<MemoryStorage> for MemoryClient<Ecal> {
    fn as_ref(&self) -> &MemoryStorage {
        self.transactor.as_ref()
    }
}

impl<Ecal: EcalHandler> AsMut<MemoryStorage> for MemoryClient<Ecal> {
    fn as_mut(&mut self) -> &mut MemoryStorage {
        self.transactor.as_mut()
    }
}

impl<Ecal: EcalHandler + Default> MemoryClient<Ecal> {
    /// Create a new instance of the memory client out of a provided storage.
    pub fn new(storage: MemoryStorage, interpreter_params: InterpreterParams) -> Self {
        Self {
            transactor: Transactor::new(storage, interpreter_params),
        }
    }
}

impl<Ecal: EcalHandler> MemoryClient<Ecal> {
    /// Create a new instance of the memory client out of a provided storage.
    pub fn from_txtor(transactor: Transactor<MemoryStorage, Script, Ecal>) -> Self {
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

#[cfg(feature = "test-helpers")]
impl<Ecal: EcalHandler + Default> From<MemoryStorage> for MemoryClient<Ecal> {
    fn from(s: MemoryStorage) -> Self {
        Self::new(s, InterpreterParams::default())
    }
}

impl<Ecal: EcalHandler> From<MemoryClient<Ecal>>
    for Transactor<MemoryStorage, Script, Ecal>
{
    fn from(client: MemoryClient<Ecal>) -> Self {
        client.transactor
    }
}

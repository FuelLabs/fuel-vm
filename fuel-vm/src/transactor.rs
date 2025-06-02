//! State machine of the interpreter.

#[cfg(any(test, feature = "test-helpers"))]
use crate::interpreter::MemoryInstance;
use crate::{
    backtrace::Backtrace,
    checked_transaction::{
        Checked,
        IntoChecked,
        Ready,
    },
    error::InterpreterError,
    interpreter::{
        CheckedMetadata,
        EcalHandler,
        ExecutableTransaction,
        Interpreter,
        InterpreterParams,
        Memory,
        NotSupportedEcal,
    },
    state::{
        ProgramState,
        StateTransition,
        StateTransitionRef,
    },
    storage::InterpreterStorage,
    verification::{
        Normal,
        Verifier,
    },
};
use fuel_tx::{
    Blob,
    Create,
    FeeParameters,
    GasCosts,
    Receipt,
    Script,
    Upgrade,
    Upload,
    field::Inputs,
};

#[derive(Debug)]
/// State machine to execute transactions and provide runtime entities on
/// demand.
///
/// Builder pattern for [`Interpreter`]. Follows the recommended `Non-consuming
/// builder`.
///
/// Based on <https://doc.rust-lang.org/1.5.0/style/ownership/builders.html#non-consuming-builders-preferred>
pub struct Transactor<M, S, Tx, Ecal = NotSupportedEcal, V = Normal>
where
    S: InterpreterStorage,
{
    interpreter: Interpreter<M, S, Tx, Ecal, V>,
    program_state: Option<ProgramState>,
    error: Option<InterpreterError<S::DataError>>,
}

impl<M, S, Tx, Ecal, V> Transactor<M, S, Tx, Ecal, V>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
    V: Verifier + Default,
{
    /// Transactor constructor
    pub fn new(memory: M, storage: S, interpreter_params: InterpreterParams) -> Self {
        Self {
            interpreter: Interpreter::<M, S, Tx, Ecal, V>::with_storage(
                memory,
                storage,
                interpreter_params,
            ),
            program_state: None,
            error: None,
        }
    }
}
impl<M, S, Tx, Ecal, V> Transactor<M, S, Tx, Ecal, V>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    /// State transition representation after the execution of a transaction.
    ///
    /// Will be `None` if the last transaction resulted in a VM panic, or if no
    /// transaction was executed.
    pub fn state_transition(&self) -> Option<StateTransitionRef<'_, Tx>> {
        self.program_state.map(|state| {
            StateTransitionRef::new(
                state,
                self.interpreter.transaction(),
                self.interpreter.receipts(),
            )
        })
    }

    /// State transition representation after the execution of a transaction.
    ///
    /// Will be `None` if the last transaction resulted in a VM panic, or if no
    /// transaction was executed.
    pub fn to_owned_state_transition(&self) -> Option<StateTransition<Tx>> {
        self.program_state.map(|state| {
            StateTransition::new(
                state,
                self.interpreter.transaction().clone(),
                self.interpreter.receipts().to_vec(),
            )
        })
    }

    /// Interpreter error representation after the execution of a transaction.
    ///
    /// Follows the same criteria as [`Self::state_transition`] to return
    /// `None`.
    ///
    /// Will be `None` if the last transaction resulted successful, or if no
    /// transaction was executed.
    pub const fn error(&self) -> Option<&InterpreterError<S::DataError>> {
        self.error.as_ref()
    }

    /// Returns true if last transaction execution was successful
    pub const fn is_success(&self) -> bool {
        !self.is_reverted()
    }

    /// Returns true if last transaction execution was erroneous
    pub const fn is_reverted(&self) -> bool {
        self.error.is_some()
            || matches!(self.program_state, Some(ProgramState::Revert(_)))
    }

    /// Result representation of the last executed transaction.
    ///
    /// Will return `None` if no transaction was executed.
    pub fn result(
        &self,
    ) -> Result<StateTransitionRef<'_, Tx>, &InterpreterError<S::DataError>> {
        let state = self.state_transition();
        let error = self.error.as_ref();

        match (state, error) {
            (Some(s), None) => Ok(s),
            (None, Some(e)) => Err(e),

            // Cover also inconsistent states such as `(Some, Some)`
            _ => Err(&InterpreterError::NoTransactionInitialized),
        }
    }

    /// Gets the interpreter.
    pub fn interpreter(&self) -> &Interpreter<M, S, Tx, Ecal, V> {
        &self.interpreter
    }

    /// Gas costs of opcodes
    pub fn gas_costs(&self) -> &GasCosts {
        self.interpreter.gas_costs()
    }

    /// Fee parameters
    pub fn fee_params(&self) -> &FeeParameters {
        self.interpreter.fee_params()
    }

    #[cfg(feature = "test-helpers")]
    /// Sets the gas price of the `Interpreter`
    pub fn set_gas_price(&mut self, gas_price: u64) {
        self.interpreter.set_gas_price(gas_price);
    }

    /// Tx memory offset
    pub fn tx_offset(&self) -> usize {
        self.interpreter.tx_offset()
    }
}

impl<M, S, Ecal, V> Transactor<M, S, Script, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
{
    /// Receipts after the execution of a transaction.
    ///
    /// Follows the same criteria as [`Self::state_transition`] to return
    /// `None`.
    pub fn receipts(&self) -> Option<&[Receipt]> {
        self.program_state
            .is_some()
            .then(|| self.interpreter.receipts())
    }

    /// Generate a backtrace when at least one receipt of `ScriptResult` was
    /// found.
    pub fn backtrace(&self) -> Option<Backtrace> {
        self.receipts()
            .and_then(|r| r.iter().find_map(Receipt::result))
            .copied()
            .map(|result| Backtrace::from_vm_error(&self.interpreter, result))
    }
}

impl<M, S, Tx, Ecal, V> Transactor<M, S, Tx, Ecal, V>
where
    S: InterpreterStorage,
{
    /// Deploys `Create` checked transactions.
    pub fn deploy(
        &mut self,
        checked: Checked<Create>,
    ) -> Result<Create, InterpreterError<S::DataError>> {
        let gas_price = self.interpreter.gas_price();
        let gas_costs = self.interpreter.gas_costs();
        let fee_params = self.interpreter.fee_params();

        let ready = checked
            .into_ready(gas_price, gas_costs, fee_params, None)
            .map_err(InterpreterError::CheckError)?;

        self.deploy_ready_tx(ready)
    }

    /// Deployt a `Ready` transaction directly instead of letting `Transactor` construct
    pub fn deploy_ready_tx(
        &mut self,
        ready_tx: Ready<Create>,
    ) -> Result<Create, InterpreterError<S::DataError>> {
        self.interpreter.deploy(ready_tx)
    }

    /// Executes `Upgrade` checked transactions.
    pub fn upgrade(
        &mut self,
        checked: Checked<Upgrade>,
    ) -> Result<Upgrade, InterpreterError<S::DataError>> {
        let gas_price = self.interpreter.gas_price();
        let gas_costs = self.interpreter.gas_costs();
        let fee_params = self.interpreter.fee_params();

        let ready = checked
            .into_ready(gas_price, gas_costs, fee_params, None)
            .map_err(InterpreterError::CheckError)?;

        self.execute_ready_upgrade_tx(ready)
    }

    /// Executes a `Ready` transaction directly instead of letting `Transactor` construct
    pub fn execute_ready_upgrade_tx(
        &mut self,
        ready_tx: Ready<Upgrade>,
    ) -> Result<Upgrade, InterpreterError<S::DataError>> {
        self.interpreter.upgrade(ready_tx)
    }

    /// Executes `Upload` checked transactions.
    pub fn upload(
        &mut self,
        checked: Checked<Upload>,
    ) -> Result<Upload, InterpreterError<S::DataError>> {
        let gas_price = self.interpreter.gas_price();
        let gas_costs = self.interpreter.gas_costs();
        let fee_params = self.interpreter.fee_params();

        let ready = checked
            .into_ready(gas_price, gas_costs, fee_params, None)
            .map_err(InterpreterError::CheckError)?;

        self.execute_ready_upload_tx(ready)
    }

    /// Executes a `Ready` transaction directly instead of letting `Transactor` construct
    pub fn execute_ready_upload_tx(
        &mut self,
        ready_tx: Ready<Upload>,
    ) -> Result<Upload, InterpreterError<S::DataError>> {
        self.interpreter.upload(ready_tx)
    }

    /// Executes `Blob` checked transactions.
    pub fn blob(
        &mut self,
        checked: Checked<Blob>,
    ) -> Result<Blob, InterpreterError<S::DataError>> {
        let gas_price = self.interpreter.gas_price();
        let gas_costs = self.interpreter.gas_costs();
        let fee_params = self.interpreter.fee_params();

        let ready = checked
            .into_ready(gas_price, gas_costs, fee_params, None)
            .map_err(InterpreterError::CheckError)?;

        self.execute_ready_blob_tx(ready)
    }

    /// Executes a `Ready` transaction directly instead of letting `Transactor` construct
    pub fn execute_ready_blob_tx(
        &mut self,
        ready_tx: Ready<Blob>,
    ) -> Result<Blob, InterpreterError<S::DataError>> {
        self.interpreter.blob(ready_tx)
    }
}

impl<M, S, Tx, Ecal, V> Transactor<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Tx: Inputs,
    <Tx as IntoChecked>::Metadata: CheckedMetadata,
    Ecal: EcalHandler,
    V: Verifier,
{
    /// Execute a transaction, and return the new state of the transactor
    pub fn transact(&mut self, tx: Checked<Tx>) -> &mut Self {
        let gas_price = self.interpreter.gas_price();
        let gas_costs = self.interpreter.gas_costs();
        let fee_params = self.interpreter.fee_params();
        let block_height = self.interpreter.context().block_height();

        let res = tx
            .into_ready(gas_price, gas_costs, fee_params, block_height)
            .map_err(InterpreterError::CheckError);
        match res {
            Ok(ready_tx) => self.transact_ready_tx(ready_tx),
            Err(e) => self.handle_error(e),
        }
    }

    /// Transact a `Ready` transaction directly instead of letting `Transactor` construct
    pub fn transact_ready_tx(&mut self, ready_tx: Ready<Tx>) -> &mut Self {
        match self.interpreter.transact(ready_tx) {
            Ok(s) => {
                self.program_state.replace(s.into());
                self.error.take();
                self
            }

            Err(e) => self.handle_error(e),
        }
    }

    fn handle_error(&mut self, error: InterpreterError<S::DataError>) -> &mut Self {
        self.program_state.take();
        self.error.replace(error);
        self
    }
}

impl<M, S, Tx, Ecal, V> From<Interpreter<M, S, Tx, Ecal, V>>
    for Transactor<M, S, Tx, Ecal, V>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
{
    fn from(interpreter: Interpreter<M, S, Tx, Ecal, V>) -> Self {
        let program_state = None;
        let error = None;

        Self {
            interpreter,
            program_state,
            error,
        }
    }
}

impl<M, S, Tx, Ecal, V> From<Transactor<M, S, Tx, Ecal, V>>
    for Interpreter<M, S, Tx, Ecal, V>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
{
    fn from(transactor: Transactor<M, S, Tx, Ecal, V>) -> Self {
        transactor.interpreter
    }
}

impl<M, S, Tx, Ecal, V> AsRef<Interpreter<M, S, Tx, Ecal, V>>
    for Transactor<M, S, Tx, Ecal, V>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
    Ecal: EcalHandler,
{
    fn as_ref(&self) -> &Interpreter<M, S, Tx, Ecal, V> {
        &self.interpreter
    }
}

impl<M, S, Tx, Ecal, V> AsRef<S> for Transactor<M, S, Tx, Ecal, V>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
{
    fn as_ref(&self) -> &S {
        self.interpreter.as_ref()
    }
}

impl<M, S, Tx, Ecal, V> AsMut<S> for Transactor<M, S, Tx, Ecal, V>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
{
    fn as_mut(&mut self) -> &mut S {
        self.interpreter.as_mut()
    }
}

#[cfg(feature = "test-helpers")]
impl<S, Tx, Ecal, V> Default for Transactor<MemoryInstance, S, Tx, Ecal, V>
where
    S: InterpreterStorage + Default,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
    V: Verifier + Default,
{
    fn default() -> Self {
        Self::new(
            MemoryInstance::new(),
            S::default(),
            InterpreterParams::default(),
        )
    }
}

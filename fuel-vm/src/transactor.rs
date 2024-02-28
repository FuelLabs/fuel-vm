//! State machine of the interpreter.

use crate::{
    backtrace::Backtrace,
    checked_transaction::{
        IntoChecked,
        PartiallyCheckedTx,
    },
    error::InterpreterError,
    interpreter::{
        CheckedMetadata,
        EcalHandler,
        ExecutableTransaction,
        Interpreter,
    },
    state::{
        ProgramState,
        StateTransition,
        StateTransitionRef,
    },
    storage::InterpreterStorage,
};

use crate::interpreter::{
    InterpreterParams,
    NotSupportedEcal,
};
use fuel_tx::{
    Create,
    GasCosts,
    Receipt,
    Script,
};

#[derive(Debug)]
/// State machine to execute transactions and provide runtime entities on
/// demand.
///
/// Builder pattern for [`Interpreter`]. Follows the recommended `Non-consuming
/// builder`.
///
/// Based on <https://doc.rust-lang.org/1.5.0/style/ownership/builders.html#non-consuming-builders-preferred>
pub struct Transactor<S, Tx, Ecal = NotSupportedEcal>
where
    S: InterpreterStorage,
{
    interpreter: Interpreter<S, Tx, Ecal>,
    program_state: Option<ProgramState>,
    error: Option<InterpreterError<S::DataError>>,
}

impl<S, Tx, Ecal> Transactor<S, Tx, Ecal>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
{
    /// Transactor constructor
    pub fn new(storage: S, interpreter_params: InterpreterParams) -> Self {
        Self {
            interpreter: Interpreter::<S, Tx, Ecal>::with_storage(
                storage,
                interpreter_params,
            ),
            program_state: None,
            error: None,
        }
    }
}
impl<'a, S, Tx, Ecal> Transactor<S, Tx, Ecal>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    /// State transition representation after the execution of a transaction.
    ///
    /// Will be `None` if the last transaction resulted in a VM panic, or if no
    /// transaction was executed.
    pub fn state_transition(&'a self) -> Option<StateTransitionRef<'a, Tx>> {
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
        &'a self,
    ) -> Result<StateTransitionRef<'a, Tx>, &InterpreterError<S::DataError>> {
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
    pub fn interpreter(&self) -> &Interpreter<S, Tx, Ecal> {
        &self.interpreter
    }

    /// Gas costs of opcodes
    pub fn gas_costs(&self) -> &GasCosts {
        self.interpreter.gas_costs()
    }

    /// Tx memory offset
    pub fn tx_offset(&self) -> usize {
        self.interpreter.tx_offset()
    }
}

impl<S, Ecal> Transactor<S, Script, Ecal>
where
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

impl<S, Tx, Ecal> Transactor<S, Tx, Ecal>
where
    S: InterpreterStorage,
{
    /// Deploys `Create` checked transactions.
    pub fn deploy(
        &mut self,
        checked: PartiallyCheckedTx<Create>,
    ) -> Result<Create, InterpreterError<S::DataError>> {
        self.interpreter.deploy(checked)
    }
}

impl<S, Tx, Ecal> Transactor<S, Tx, Ecal>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    <Tx as IntoChecked>::Metadata: CheckedMetadata,
    Ecal: EcalHandler,
{
    /// Execute a transaction, and return the new state of the transactor
    pub fn transact(&mut self, tx: PartiallyCheckedTx<Tx>) -> &mut Self {
        let gas_price = self.interpreter.gas_price();
        let gas_costs = self.interpreter.gas_costs();
        let fee_params = self.interpreter.fee_params();

        match tx
            .into_fully_checked(gas_price, &gas_costs, &fee_params)
            .map_err(InterpreterError::CheckError)
            .and_then(|immutable| self.interpreter.transact(immutable))
        {
            Ok(s) => {
                self.program_state.replace(s.into());
                self.error.take();
            }

            Err(e) => {
                self.program_state.take();
                self.error.replace(e);
            }
        }
        self
    }
}

impl<S, Tx, Ecal> From<Interpreter<S, Tx, Ecal>> for Transactor<S, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
{
    fn from(interpreter: Interpreter<S, Tx, Ecal>) -> Self {
        let program_state = None;
        let error = None;

        Self {
            interpreter,
            program_state,
            error,
        }
    }
}

impl<S, Tx, Ecal> From<Transactor<S, Tx, Ecal>> for Interpreter<S, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
{
    fn from(transactor: Transactor<S, Tx, Ecal>) -> Self {
        transactor.interpreter
    }
}

impl<S, Tx, Ecal> AsRef<Interpreter<S, Tx, Ecal>> for Transactor<S, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
    Ecal: EcalHandler,
{
    fn as_ref(&self) -> &Interpreter<S, Tx, Ecal> {
        &self.interpreter
    }
}

impl<S, Tx, Ecal> AsRef<S> for Transactor<S, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
{
    fn as_ref(&self) -> &S {
        self.interpreter.as_ref()
    }
}

impl<S, Tx, Ecal> AsMut<S> for Transactor<S, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
{
    fn as_mut(&mut self) -> &mut S {
        self.interpreter.as_mut()
    }
}

impl<S, Tx, Ecal> Default for Transactor<S, Tx, Ecal>
where
    S: InterpreterStorage + Default,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
{
    fn default() -> Self {
        Self::new(S::default(), InterpreterParams::default())
    }
}

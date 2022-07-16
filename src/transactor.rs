//! State machine of the interpreter.

use crate::backtrace::Backtrace;
use crate::error::InterpreterError;
use crate::interpreter::Interpreter;
use crate::state::StateTransitionRef;
use crate::storage::InterpreterStorage;

use fuel_tx::{CheckedTransaction, ConsensusParameters, Receipt};

use std::{mem, slice};

#[derive(Debug)]
/// State machine to execute transactions and provide runtime entities on
/// demand.
///
/// Builder pattern for [`Interpreter`]. Follows the recommended `Non-consuming
/// builder`.
///
/// Based on <https://doc.rust-lang.org/1.5.0/style/ownership/builders.html#non-consuming-builders-preferred>
pub struct Transactor<'a, S> {
    interpreter: Interpreter<S>,
    state_transition: Option<StateTransitionRef<'a>>,
    error: Option<InterpreterError>,
}

impl<'a, S> Transactor<'a, S> {
    /// Transactor constructor
    pub fn new(storage: S, params: ConsensusParameters) -> Self {
        Interpreter::with_storage(storage, params).into()
    }

    /// State transition representation after the execution of a transaction.
    ///
    /// Will be `None` if the last transaction resulted in a VM panic, or if no
    /// transaction was executed.
    pub const fn state_transition(&self) -> Option<&StateTransitionRef<'a>> {
        self.state_transition.as_ref()
    }

    /// Receipts after the execution of a transaction.
    ///
    /// Follows the same criteria as [`Self::state_transition`] to return
    /// `None`.
    pub fn receipts(&self) -> Option<&[Receipt]> {
        // TODO improve implementation without changing signature

        self.state_transition().map(|s| {
            let receipts = s.receipts();

            let ptr = receipts.as_ptr();
            let len = receipts.len();

            // Safety: StateTransitionRef is bound to 'a
            //
            // We can enforce this as a compiler rule by requiring `receipts(&'a self)`, but
            // then the consumers of this API will face unnecessary lifetime complexity
            unsafe { slice::from_raw_parts(ptr, len) }
        })
    }

    /// Interpreter error representation after the execution of a transaction.
    ///
    /// Follows the same criteria as [`Self::state_transition`] to return
    /// `None`.
    ///
    /// Will be `None` if the last transaction resulted successful, or if no
    /// transaction was executed.
    pub const fn error(&self) -> Option<&InterpreterError> {
        self.error.as_ref()
    }

    /// Generate a backtrace when at least one receipt of `ScriptResult` was
    /// found.
    pub fn backtrace(&self) -> Option<Backtrace> {
        self.receipts()
            .map(|r| r.iter().find_map(Receipt::result))
            .flatten()
            .copied()
            .map(|result| Backtrace::from_vm_error(&self.interpreter, result))
    }

    /// Returns true if last transaction execution was successful
    pub const fn is_success(&self) -> bool {
        self.state_transition.is_some()
    }

    /// Returns true if last transaction execution was erroneous
    pub const fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Result representation of the last executed transaction.
    ///
    /// Will return `None` if no transaction was executed.
    pub fn result(&self) -> Result<&StateTransitionRef<'a>, &InterpreterError> {
        let state = self.state_transition.as_ref();
        let error = self.error.as_ref();

        match (state, error) {
            (Some(s), None) => Ok(s),
            (None, Some(e)) => Err(e),

            // Cover also inconsistent states such as `(Some, Some)`
            _ => Err(&InterpreterError::NoTransactionInitialized),
        }
    }

    /// Convert this transaction into the underlying VM instance.
    ///
    /// This isn't a two-way operation since if you convert this instance into
    /// the raw VM, then you lose the transactor state.
    pub fn interpreter(self) -> Interpreter<S> {
        self.into()
    }

    /// Consensus parameters
    pub const fn params(&self) -> &ConsensusParameters {
        self.interpreter.params()
    }

    /// Tx memory offset
    pub const fn tx_offset(&self) -> usize {
        self.interpreter.tx_offset()
    }
}

impl<S> Transactor<'_, S>
where
    S: InterpreterStorage,
{
    /// Execute a transaction, and return the new state of the transactor
    pub fn transact(&mut self, tx: CheckedTransaction) -> &mut Self {
        match self.interpreter.transact(tx) {
            Ok(s) => {
                // Safety: cast `StateTransitionRef<'_>` to `StateTransitionRef<'a>` since it
                // was generated with the same lifetime of `self.interpreter`
                //
                // `self.interpreter` is bound to `'a` since its bound to `self`
                let s = unsafe { mem::transmute(s) };

                self.state_transition.replace(s);
                self.error.take();
            }

            Err(e) => {
                self.state_transition.take();
                self.error.replace(e);
            }
        }

        self
    }
}

impl<S> From<Interpreter<S>> for Transactor<'_, S> {
    fn from(interpreter: Interpreter<S>) -> Self {
        let state_transition = None;
        let error = None;

        Self {
            interpreter,
            state_transition,
            error,
        }
    }
}

impl<S> From<Transactor<'_, S>> for Interpreter<S> {
    fn from(transactor: Transactor<S>) -> Self {
        transactor.interpreter
    }
}

impl<S> AsRef<Interpreter<S>> for Transactor<'_, S> {
    fn as_ref(&self) -> &Interpreter<S> {
        &self.interpreter
    }
}

impl<S> AsRef<S> for Transactor<'_, S> {
    fn as_ref(&self) -> &S {
        self.interpreter.as_ref()
    }
}

impl<S> AsMut<S> for Transactor<'_, S> {
    fn as_mut(&mut self) -> &mut S {
        self.interpreter.as_mut()
    }
}

impl<S> Default for Transactor<'_, S>
where
    S: Default,
{
    fn default() -> Self {
        Self::new(Default::default(), Default::default())
    }
}

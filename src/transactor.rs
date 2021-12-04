//! Builder pattern for [`Interpreter`]
//!
//! Based on <https://doc.rust-lang.org/1.5.0/style/ownership/builders.html#non-consuming-builders-preferred>
//!
//! Follows the recommended `Non-consuming builder`.

use crate::backtrace::Backtrace;
use crate::error::InterpreterError;
use crate::interpreter::Interpreter;
use crate::state::StateTransitionRef;
use crate::storage::InterpreterStorage;

use fuel_tx::{Receipt, Transaction};

use std::slice;

#[derive(Debug, Clone)]
pub struct Transactor<'a, S> {
    interpreter: Interpreter<S>,
    state_transition: Option<StateTransitionRef<'a>>,
    error: Option<InterpreterError>,
}

impl<'a, S> Transactor<'a, S> {
    pub fn new(storage: S) -> Self {
        Interpreter::with_storage(storage).into()
    }

    pub const fn state_transition(&self) -> Option<&StateTransitionRef<'a>> {
        self.state_transition.as_ref()
    }

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

    pub const fn is_success(&self) -> bool {
        self.error.is_none()
    }

    pub const fn is_error(&self) -> bool {
        !self.is_success()
    }

    pub fn result(&self) -> Result<StateTransitionRef<'a>, InterpreterError> {
        match (self.state_transition, &self.error) {
            (Some(s), None) => Ok(s),
            (None, Some(e)) => Err(e.clone()),

            // Cover also inconsistent states such as `(Some, Some)`
            _ => Err(InterpreterError::NoTransactionInitialized),
        }
    }

    pub fn interpreter(self) -> Interpreter<S> {
        self.into()
    }
}

impl<'a, S> Transactor<'a, S>
where
    S: InterpreterStorage,
{
    pub fn transact(&mut self, tx: Transaction) -> &'a mut Self {
        let slf: *mut Self = self;

        // Safety: the compiler isn't aware that `'a` encompasses `Interpreter` as well
        //
        // This is a safe call because both `Interpreter` and `StateTransitionRef` are
        // owned by `'a` via `Transactor`
        unsafe {
            match slf.as_mut().unwrap().interpreter.transact(tx) {
                Ok(s) => {
                    self.state_transition.replace(s);
                    self.error.take();

                    (self as *mut Self).as_mut().unwrap()
                }

                Err(e) => {
                    self.state_transition.take();
                    self.error.replace(e);

                    (self as *mut Self).as_mut().unwrap()
                }
            }
        }
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
        Self::new(Default::default())
    }
}

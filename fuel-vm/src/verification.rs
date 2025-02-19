//! Stategies for verifying the correctness of the VM execution.
//! The default strategy, [`Panic`], simply panics on failed verification.
//! Alternative strategy, [`AttemptContinue`], continues execution and collects all
//! errors.

use fuel_asm::Instruction;
use fuel_tx::PanicInstruction;

use crate::{
    error::RuntimeError,
    prelude::Interpreter,
    storage::InterpreterStorage,
};

/// What to do when verification fails.
pub trait Verifier<M, S, Tx, Ecal>
where
    Self: Sized,
    S: InterpreterStorage,
{
    /// Handle an error during execution
    fn on_error(
        vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
        instruction: Instruction,
        err: &RuntimeError<S::DataError>,
    ) -> OnErrorAction;
}

/// What should be done after encountering an error.
#[derive(Debug, Copy, Clone)]
pub enum OnErrorAction {
    /// The VM terminates via panic. This is the default behavior.
    Terminate,
    /// Continue execution as if nothing happened.
    Continue,
}

/// Panic on failed verification. This is the default verification strategy.
#[derive(Debug, Copy, Clone, Default)]
pub struct Panic;

impl<M, S, Tx, Ecal> Verifier<M, S, Tx, Ecal> for Panic
where
    Self: Sized,
    S: InterpreterStorage,
{
    fn on_error(
        _vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
        _instruction: Instruction,
        _err: &RuntimeError<S::DataError>,
    ) -> OnErrorAction {
        OnErrorAction::Terminate
    }
}

/// Continue execution on failed verification, storing all encountered errors.
/// This is useful for collecting multiple errors in a single run.
#[derive(Debug, Clone, Default)]
pub struct AttemptContinue {
    /// Validation encountered during execution.
    pub errors: Vec<PanicInstruction>,
}

impl<M, S, Tx, Ecal> Verifier<M, S, Tx, Ecal> for AttemptContinue
where
    Self: Sized,
    S: InterpreterStorage,
{
    fn on_error(
        vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
        instruction: Instruction,
        err: &RuntimeError<S::DataError>,
    ) -> OnErrorAction {
        OnErrorAction::Terminate
    }
}

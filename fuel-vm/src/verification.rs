//! Stategies for verifying the correctness of the VM execution.
//! The default strategy, [`Panic`], simply panics on failed verification.
//! Alternative strategy, [`AttemptContinue`], continues execution and collects multiple
//! errors.

use fuel_asm::Instruction;
use fuel_tx::ContractId;

use crate::{
    error::RuntimeError,
    prelude::Interpreter,
    storage::InterpreterStorage,
};

/// Do not allow outside implementations for the Verifier, so that it's not a breaking
/// change to modify it.
trait Seal {}

/// What to do when verification fails.
#[allow(private_bounds)] // For selaed trait
pub trait Verifier<M, S, Tx, Ecal>
where
    Self: Sized + Seal,
    S: InterpreterStorage,
{
    /// Handle an error after an instruction has been run
    fn on_instruction_error(
        _vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
        _instruction: Instruction,
        _err: &RuntimeError<S::DataError>,
    ) -> OnErrorAction {
        OnErrorAction::Terminate
    }

    /// Handle an error after a contract is missing from the inputs
    fn on_contract_not_in_inputs(&mut self, _contract_id: ContractId) -> OnErrorAction {
        OnErrorAction::Terminate
    }
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
}

impl Seal for Panic {}

/// With some subset of errors it's possible to continue execution,
/// allowing the collection of multiple errors during a single run.
#[derive(Debug, Clone, Default)]
pub struct AttemptContinue {
    /// Contracts that were called but not in the inputs
    pub missing_contract_inputs: Vec<ContractId>,
}

impl<M, S, Tx, Ecal> Verifier<M, S, Tx, Ecal> for AttemptContinue
where
    Self: Sized,
    S: InterpreterStorage,
{
    fn on_contract_not_in_inputs(&mut self, contract_id: ContractId) -> OnErrorAction {
        self.missing_contract_inputs.push(contract_id);
        OnErrorAction::Continue
    }
}

impl Seal for AttemptContinue {}

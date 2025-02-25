//! Stategies for verifying the correctness of the VM execution.
//! The default strategy, [`Panic`], simply panics on failed verification.

use alloc::collections::BTreeSet;

use fuel_tx::{
    ContractId,
    PanicReason,
};

use crate::{
    error::PanicOrBug,
    interpreter::PanicContext,
    storage::InterpreterStorage,
};

/// Do not allow outside implementations for the Verifier, so that it's not a breaking
/// change to modify it.
trait Seal {}

/// What to do when verification fails.
#[allow(private_bounds)] // For selaed trait
pub trait Verifier<S>
where
    Self: Sized + Seal,
    S: InterpreterStorage,
{
    /// Handle an error after a contract is missing from the inputs
    #[allow(private_interfaces)] // PanicContext is an internal type, so this isn't callable by external code
    fn check_contract_in_inputs(
        &mut self,
        panic_context: &mut PanicContext,
        input_contracts: &BTreeSet<ContractId>,
        contract_id: &ContractId,
    ) -> Result<(), PanicOrBug>;
}

/// Panic on failed verification. This is the default verification strategy.
#[derive(Debug, Copy, Clone, Default)]
pub struct Panic;

impl<S> Verifier<S> for Panic
where
    Self: Sized,
    S: InterpreterStorage,
{
    #[allow(private_interfaces)]
    fn check_contract_in_inputs(
        &mut self,
        panic_context: &mut PanicContext,
        input_contracts: &BTreeSet<ContractId>,
        contract_id: &ContractId,
    ) -> Result<(), PanicOrBug> {
        if input_contracts.contains(contract_id) {
            Ok(())
        } else {
            *panic_context = PanicContext::ContractId(*contract_id);
            Err(PanicReason::ContractNotInInputs.into())
        }
    }
}

impl Seal for Panic {}

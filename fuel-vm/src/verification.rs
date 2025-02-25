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
};

/// Do not allow outside implementations for the Verifier, so that it's not a breaking
/// change to modify it.
trait Seal {}

/// What to do when verification fails.
#[allow(private_bounds)] // For selaed trait
pub trait Verifier
where
    Self: Sized + Seal,
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

/// The default verification strategy.
/// Performs the standard verification checks and panics on failure.
#[derive(Debug, Copy, Clone, Default)]
pub struct Normal;

impl Verifier for Normal
where
    Self: Sized,
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

impl Seal for Normal {}

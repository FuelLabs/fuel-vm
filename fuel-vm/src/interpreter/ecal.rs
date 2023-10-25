//! See `fuel-vm/examples/external.rs` for example usage.

use fuel_asm::{
    PanicReason,
    RegId,
};

use crate::{
    constraints::reg_key::{
        split_registers,
        SystemRegisters,
    },
    error::SimpleResult,
};

use super::{
    internal::inc_pc,
    Interpreter,
};

/// ECAL opcode handler
pub trait EcalHandler: Default + Clone + Copy
where
    Self: Sized,
{
    /// Whether to increment PC after executing ECAL. If this is false,
    /// the handler must increment PC itself.
    const INC_PC: bool = true;

    /// ECAL opcode handler
    fn ecal<S, Tx>(
        vm: &mut Interpreter<S, Self, Tx>,
        _: RegId,
        _: RegId,
        _: RegId,
        _: RegId,
    ) -> SimpleResult<()>;
}

/// Default ECAL opcode handler function, which charges for `noop` and does nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopEcal;

/// Default ECAL opcode handler function, which charges for `noop` and does nothing.
impl EcalHandler for NoopEcal {
    fn ecal<S, Tx>(
        vm: &mut Interpreter<S, Self, Tx>,
        _: RegId,
        _: RegId,
        _: RegId,
        _: RegId,
    ) -> SimpleResult<()> {
        vm.gas_charge(vm.gas_costs().noop)
    }
}

/// ECAL is not allowed in predicates
#[derive(Debug, Clone, Copy, Default)]
pub struct PredicateErrorEcal;

/// ECAL is not allowed in predicates
impl EcalHandler for PredicateErrorEcal {
    fn ecal<S, Tx>(
        _vm: &mut Interpreter<S, Self, Tx>,
        _: RegId,
        _: RegId,
        _: RegId,
        _: RegId,
    ) -> SimpleResult<()> {
        Err(PanicReason::ContractInstructionNotAllowed)?
    }
}

/// ECAL opcode handler cannot be used in this context
#[derive(Debug, Clone, Copy, Default)]
pub struct UnreachableEcal;

impl EcalHandler for UnreachableEcal {
    fn ecal<S, Tx>(
        _vm: &mut Interpreter<S, Self, Tx>,
        _: RegId,
        _: RegId,
        _: RegId,
        _: RegId,
    ) -> SimpleResult<()> {
        unreachable!("ECAL cannot be used in this part of the VM")
    }
}

impl<S, Ecal, Tx> Interpreter<S, Ecal, Tx>
where
    Ecal: EcalHandler,
{
    /// Executes ECAL opcode handler function and increments PC
    pub(crate) fn external_call(
        &mut self,
        a: RegId,
        b: RegId,
        c: RegId,
        d: RegId,
    ) -> SimpleResult<()> {
        Ecal::ecal(self, a, b, c, d)?;
        let (SystemRegisters { pc, .. }, _) = split_registers(&mut self.registers);
        if Ecal::INC_PC {
            Ok(inc_pc(pc)?)
        } else {
            Ok(())
        }
    }
}

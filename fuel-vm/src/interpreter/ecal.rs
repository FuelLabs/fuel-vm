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
    interpreter::NotSupportedEcal,
};

use super::{
    internal::inc_pc,
    Interpreter,
    Memory,
};

/// ECAL opcode handler
pub trait EcalHandler: Clone
where
    Self: Sized,
{
    /// Whether to increment PC after executing ECAL. If this is false,
    /// the handler must increment PC itself.
    const INC_PC: bool = true;

    /// ECAL opcode handler
    fn ecal<M, S, Tx, OnVerifyError>(
        vm: &mut Interpreter<M, S, Tx, Self, OnVerifyError>,
        a: RegId,
        b: RegId,
        c: RegId,
        d: RegId,
    ) -> SimpleResult<()>
    where
        M: Memory;
}

/// Default ECAL opcode handler function, which just errors immediately.
impl EcalHandler for NotSupportedEcal {
    fn ecal<M, S, Tx, OnVerifyError>(
        _: &mut Interpreter<M, S, Tx, Self, OnVerifyError>,
        _: RegId,
        _: RegId,
        _: RegId,
        _: RegId,
    ) -> SimpleResult<()> {
        Err(PanicReason::EcalError)?
    }
}

/// ECAL is not allowed in predicates
#[derive(Debug, Clone, Copy, Default)]
pub struct PredicateErrorEcal;

/// ECAL is not allowed in predicates
impl EcalHandler for PredicateErrorEcal {
    fn ecal<M, S, Tx, OnVerifyError>(
        _vm: &mut Interpreter<M, S, Tx, Self, OnVerifyError>,
        _: RegId,
        _: RegId,
        _: RegId,
        _: RegId,
    ) -> SimpleResult<()> {
        Err(PanicReason::ContractInstructionNotAllowed)?
    }
}

impl<M, S, Tx, Ecal, OnVerifyError> Interpreter<M, S, Tx, Ecal, OnVerifyError>
where
    M: Memory,
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

impl<M, S, Tx, Ecal, OnVerifyError> Interpreter<M, S, Tx, Ecal, OnVerifyError>
where
    Ecal: EcalHandler,
{
    /// Read access to the ECAL state
    pub fn ecal_state(&self) -> &Ecal {
        &self.ecal_state
    }

    /// Write access to the ECAL state
    pub fn ecal_state_mut(&mut self) -> &mut Ecal {
        &mut self.ecal_state
    }
}

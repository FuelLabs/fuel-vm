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
    fn ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        a: RegId,
        b: RegId,
        c: RegId,
        d: RegId,
    ) -> SimpleResult<()>;
}

/// Default ECAL opcode handler function, which charges for `noop` and does nothing.
impl EcalHandler for NotSupportedEcal {
    fn ecal<S, Tx>(
        _: &mut Interpreter<S, Tx, Self>,
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
    fn ecal<S, Tx>(
        _vm: &mut Interpreter<S, Tx, Self>,
        _: RegId,
        _: RegId,
        _: RegId,
        _: RegId,
    ) -> SimpleResult<()> {
        Err(PanicReason::ContractInstructionNotAllowed)?
    }
}

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
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

    /// Read access to the ECAL state
    pub fn ecal_state(&self) -> &Ecal {
        &self.ecal_state
    }

    /// Write access to the ECAL state
    pub fn ecal_state_mut(&mut self) -> &mut Ecal {
        &mut self.ecal_state
    }

    /// Replace the ECAL state and handler.
    /// Useful when the handler does not implement `Default`.
    pub fn with_ecal<NewEcal: EcalHandler>(
        self,
        new_handler: NewEcal,
    ) -> Interpreter<S, Tx, NewEcal> {
        Interpreter {
            registers: self.registers,
            memory: self.memory,
            frames: self.frames,
            receipts: self.receipts,
            tx: self.tx,
            initial_balances: self.initial_balances,
            storage: self.storage,
            debugger: self.debugger,
            context: self.context,
            balances: self.balances,
            profiler: self.profiler,
            interpreter_params: self.interpreter_params,
            panic_context: self.panic_context,
            ecal_state: new_handler,
        }
    }
}

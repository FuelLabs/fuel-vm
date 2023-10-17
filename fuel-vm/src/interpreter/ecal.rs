//! See `fuel-vm/examples/external.rs` for example usage.

use fuel_asm::RegId;
use fuel_tx::DependentCost;
use fuel_types::Word;

use crate::{
    call::CallFrame,
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

/// Accessing the VM state from ECAL instruction handler is done through this trait.
pub trait EcalAccess {
    // Accessors

    fn memory(&self) -> &[u8];
    fn memory_mut(&mut self) -> &mut [u8];

    fn registers(&self) -> &[Word];
    fn registers_mut(&mut self) -> &mut [Word];

    fn call_stack(&self) -> &[CallFrame];

    fn gas_charge(&mut self, amount: Word) -> SimpleResult<()>;
    fn dependent_gas_charge(
        &mut self,
        gas_cost: DependentCost,
        arg: Word,
    ) -> SimpleResult<()>;
    fn gas_costs(&self) -> &fuel_tx::GasCosts;

    // Helper methods

    fn allocate(&mut self, size: Word) -> SimpleResult<()>;
}

impl<S, Tx> EcalAccess for Interpreter<S, Tx> {
    fn memory(&self) -> &[u8] {
        self.memory.as_slice()
    }

    fn memory_mut(&mut self) -> &mut [u8] {
        self.memory.as_mut()
    }

    fn registers(&self) -> &[Word] {
        &self.registers
    }

    fn registers_mut(&mut self) -> &mut [Word] {
        &mut self.registers
    }

    fn call_stack(&self) -> &[CallFrame] {
        self.frames.as_slice()
    }

    fn gas_charge(&mut self, amount: Word) -> SimpleResult<()> {
        self.gas_charge(amount)
    }

    fn dependent_gas_charge(
        &mut self,
        gas_cost: DependentCost,
        arg: Word,
    ) -> SimpleResult<()> {
        self.dependent_gas_charge(gas_cost, arg)
    }

    fn gas_costs(&self) -> &fuel_tx::GasCosts {
        &self.interpreter_params.gas_costs
    }

    fn allocate(&mut self, size: Word) -> SimpleResult<()> {
        self.malloc(size)
    }
}

/// ECAL opcode handler function type
pub type EcalFn = fn(&mut dyn EcalAccess, RegId, RegId, RegId, RegId) -> SimpleResult<()>;

/// Default ECAL opcode handler function, which charges for `noop` and does nothing.
fn noop_ecall(
    vm: &mut dyn EcalAccess,
    _: RegId,
    _: RegId,
    _: RegId,
    _: RegId,
) -> SimpleResult<()> {
    vm.gas_charge(vm.gas_costs().noop)
}

impl<S, Tx> Interpreter<S, Tx> {
    pub(crate) const DEFAULT_ECAL: EcalFn = noop_ecall;

    /// Sets ECAL opcode handler function
    pub fn set_ecal(&mut self, ecal_function: EcalFn) {
        self.ecal_function = ecal_function;
    }

    /// Resets ECAL opcode handler function back to default noop
    pub fn reset_ecal(&mut self) {
        self.set_ecal(Self::DEFAULT_ECAL);
    }

    /// Executes ECAL opcode handler function and increments PC
    pub(crate) fn external_call(
        &mut self,
        a: RegId,
        b: RegId,
        c: RegId,
        d: RegId,
    ) -> SimpleResult<()> {
        (self.ecal_function)(self, a, b, c, d)?;
        let (SystemRegisters { pc, .. }, _) = split_registers(&mut self.registers);
        Ok(inc_pc(pc)?)
    }
}

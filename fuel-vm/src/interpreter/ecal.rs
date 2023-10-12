use fuel_asm::RegId;
use fuel_tx::DependentCost;
use fuel_types::Word;

use crate::{
    call::CallFrame,
    error::SimpleResult,
};

use super::Interpreter;

pub trait EcalAccess {
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
}

pub type EcalFn = fn(&mut dyn EcalAccess, RegId, RegId, RegId, RegId) -> SimpleResult<()>;

fn noop_ecall(
    vm: &mut dyn EcalAccess,
    _: RegId,
    _: RegId,
    _: RegId,
    _: RegId,
) -> SimpleResult<()> {
    vm.gas_charge(vm.gas_costs().noop)?;
    Ok(())
}

impl<S, Tx> Interpreter<S, Tx> {
    pub(crate) const DEFAULT_ECAL: EcalFn = noop_ecall;

    pub(crate) fn external_call(
        &mut self,
        a: RegId,
        b: RegId,
        c: RegId,
        d: RegId,
    ) -> SimpleResult<()> {
        (self.ecal_function)(self, a, b, c, d)
    }
}

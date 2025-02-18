use super::Interpreter;
use crate::{
    constraints::reg_key::*,
    error::SimpleResult,
    prelude::{
        Bug,
        BugVariant,
    },
};

use fuel_asm::{
    PanicReason,
    RegId,
};
use fuel_tx::DependentCost;
use fuel_types::{
    Word,
};

#[cfg(test)]
mod tests;

impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal> {
    /// Global remaining gas amount
    pub fn remaining_gas(&self) -> Word {
        self.registers[RegId::GGAS]
    }

    /// Sets the amount of gas available for execution to both CGAS and GGAS.
    /// Only useful in contexts where CGAS and GGAS are the same,
    /// i.e. predicates and testing.
    pub(crate) fn set_gas(&mut self, gas: Word) {
        self.registers[RegId::GGAS] = gas;
        self.registers[RegId::CGAS] = gas;
    }

    pub(crate) fn dependent_gas_charge(
        &mut self,
        gas_cost: DependentCost,
        arg: Word,
    ) -> SimpleResult<()> {
        let SystemRegisters {
            ggas, cgas, ..
        } = split_registers(&mut self.registers).0;
        dependent_gas_charge(cgas, ggas, gas_cost, arg)
    }

    pub(crate) fn dependent_gas_charge_without_base(
        &mut self,
        gas_cost: DependentCost,
        arg: Word,
    ) -> SimpleResult<()> {
        let SystemRegisters {
            ggas, cgas, ..
        } = split_registers(&mut self.registers).0;
        dependent_gas_charge_without_base(cgas, ggas, gas_cost, arg)
    }

    /// Do a gas charge with the given amount, panicing when running out of gas.
    pub fn gas_charge(&mut self, gas: Word) -> SimpleResult<()> {
        let SystemRegisters {
            ggas, cgas, ..
        } = split_registers(&mut self.registers).0;

        gas_charge(cgas, ggas, gas)
    }
}

pub(crate) fn dependent_gas_charge_without_base(
    mut cgas: RegMut<CGAS>,
    ggas: RegMut<GGAS>,
    gas_cost: DependentCost,
    arg: Word,
) -> SimpleResult<()> {
    let cost = gas_cost.resolve_without_base(arg);
    gas_charge_inner(cgas.as_mut(), ggas, cost)
}

pub(crate) fn dependent_gas_charge(
    mut cgas: RegMut<CGAS>,
    ggas: RegMut<GGAS>,
    gas_cost: DependentCost,
    arg: Word,
) -> SimpleResult<()> {
    let cost = gas_cost.resolve(arg);
    gas_charge_inner(cgas.as_mut(), ggas, cost)
}

pub(crate) fn gas_charge(
    cgas: RegMut<CGAS>,
    ggas: RegMut<GGAS>,
    gas: Word,
) -> SimpleResult<()> {
    gas_charge_inner(cgas, ggas, gas)
}

fn gas_charge_inner(
    mut cgas: RegMut<CGAS>,
    mut ggas: RegMut<GGAS>,
    gas: Word,
) -> SimpleResult<()> {
    if *cgas > *ggas {
        Err(Bug::new(BugVariant::GlobalGasLessThanContext).into())
    } else if gas > *cgas {
        *ggas = (*ggas)
            .checked_sub(*cgas)
            .ok_or_else(|| Bug::new(BugVariant::GlobalGasUnderflow))?;
        *cgas = 0;

        Err(PanicReason::OutOfGas.into())
    } else {
        *cgas = (*cgas)
            .checked_sub(gas)
            .ok_or_else(|| Bug::new(BugVariant::ContextGasUnderflow))?;
        *ggas = (*ggas)
            .checked_sub(gas)
            .ok_or_else(|| Bug::new(BugVariant::GlobalGasUnderflow))?;

        Ok(())
    }
}

use super::Interpreter;
use crate::{
    constraints::reg_key::*,
    error::{
        PanicOrBug,
        SimpleResult,
    },
    prelude::{
        Bug,
        BugVariant,
    },
    profiler::Profiler,
};

use fuel_asm::{
    PanicReason,
    RegId,
};
use fuel_tx::DependentCost;
use fuel_types::{
    ContractId,
    Word,
};

#[cfg(test)]
mod tests;

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal> {
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
        let current_contract = self.contract_id();
        let SystemRegisters {
            pc, ggas, cgas, is, ..
        } = split_registers(&mut self.registers).0;
        let profiler = ProfileGas {
            pc: pc.as_ref(),
            is: is.as_ref(),
            current_contract,
            profiler: &mut self.profiler,
        };
        dependent_gas_charge(cgas, ggas, profiler, gas_cost, arg)
    }

    /// Do a gas charge with the given amount, panicing when running out of gas.
    pub fn gas_charge(&mut self, gas: Word) -> SimpleResult<()> {
        let current_contract = self.contract_id();
        let SystemRegisters {
            pc, ggas, cgas, is, ..
        } = split_registers(&mut self.registers).0;

        let profiler = ProfileGas {
            pc: pc.as_ref(),
            is: is.as_ref(),
            current_contract,
            profiler: &mut self.profiler,
        };
        gas_charge(cgas, ggas, profiler, gas)
    }
}

pub(crate) fn dependent_gas_charge(
    mut cgas: RegMut<CGAS>,
    ggas: RegMut<GGAS>,
    mut profiler: ProfileGas<'_>,
    gas_cost: DependentCost,
    arg: Word,
) -> SimpleResult<()> {
    if gas_cost.dep_per_unit == 0 {
        gas_charge(cgas, ggas, profiler, gas_cost.base)
    } else {
        let cost = dependent_gas_charge_inner(cgas.as_mut(), ggas, gas_cost, arg)?;
        profiler.profile(cgas.as_ref(), cost);
        Ok(())
    }
}

fn dependent_gas_charge_inner(
    cgas: RegMut<CGAS>,
    ggas: RegMut<GGAS>,
    gas_cost: DependentCost,
    arg: Word,
) -> Result<Word, PanicOrBug> {
    let cost = gas_cost
        .base
        .saturating_add(arg.saturating_div(gas_cost.dep_per_unit));
    gas_charge_inner(cgas, ggas, cost).map(|_| cost)
}

pub(crate) fn gas_charge(
    cgas: RegMut<CGAS>,
    ggas: RegMut<GGAS>,
    mut profiler: ProfileGas<'_>,
    gas: Word,
) -> SimpleResult<()> {
    profiler.profile(cgas.as_ref(), gas);
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

#[allow(dead_code)]
pub(crate) struct ProfileGas<'a> {
    pub pc: Reg<'a, PC>,
    pub is: Reg<'a, IS>,
    pub current_contract: Option<ContractId>,
    pub profiler: &'a mut Profiler,
}

impl<'a> ProfileGas<'a> {
    #[allow(unused_variables)]
    pub(crate) fn profile(&mut self, cgas: Reg<CGAS>, gas: Word) {
        #[cfg(feature = "profile-coverage")]
        {
            let location =
                super::current_location(self.current_contract, self.pc, self.is);
            self.profiler.set_coverage(location);
        }

        #[cfg(feature = "profile-gas")]
        {
            let gas_use = gas.min(*cgas);
            let location =
                super::current_location(self.current_contract, self.pc, self.is);
            self.profiler.add_gas(location, gas_use);
        }
    }
}

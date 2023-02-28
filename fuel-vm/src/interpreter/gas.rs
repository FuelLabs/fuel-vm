use super::Interpreter;
use crate::arith;
use crate::constraints::reg_key::*;
use crate::constraints::ProfileCapture;
use crate::error::RuntimeError;
use crate::gas::DependentCost;

use fuel_asm::PanicReason;
use fuel_asm::RegId;
use fuel_types::ContractId;
use fuel_types::Word;

#[cfg(test)]
mod tests;

impl<S, Tx> Interpreter<S, Tx> {
    pub(crate) fn remaining_gas(&self) -> Word {
        self.registers[RegId::GGAS]
    }

    /// Sets the remaining amout of gas to both CGAS and GGAS.
    /// Only useful in contexts where CGAS and GGAS are the same,
    /// i.e. predicates and testing.
    pub(crate) fn set_remaining_gas(&mut self, gas: Word) {
        self.registers[RegId::GGAS] = gas;
        self.registers[RegId::CGAS] = gas;
    }

    pub(crate) fn dependent_gas_charge(&mut self, gas_cost: DependentCost, arg: Word) -> Result<(), RuntimeError> {
        let current_contract = self.contract_id();
        let SystemRegisters { pc, ggas, cgas, is, .. } = split_registers(&mut self.registers).0;
        let profiler = ProfileGas {
            pc: pc.as_ref(),
            is: is.as_ref(),
            current_contract,
            profiler: &mut self.profiler,
        };
        dependent_gas_charge(cgas, ggas, profiler, gas_cost, arg)
    }

    pub(crate) fn gas_charge(&mut self, gas: Word) -> Result<(), RuntimeError> {
        let current_contract = self.contract_id();
        let SystemRegisters { pc, ggas, cgas, is, .. } = split_registers(&mut self.registers).0;

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
) -> Result<(), RuntimeError> {
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
) -> Result<Word, RuntimeError> {
    let cost = gas_cost.base.saturating_add(arg.saturating_div(gas_cost.dep_per_unit));
    gas_charge_inner(cgas, ggas, cost).map(|_| cost)
}

pub(crate) fn gas_charge(
    cgas: RegMut<CGAS>,
    ggas: RegMut<GGAS>,
    mut profiler: ProfileGas<'_>,
    gas: Word,
) -> Result<(), RuntimeError> {
    profiler.profile(cgas.as_ref(), gas);
    gas_charge_inner(cgas, ggas, gas)
}

fn gas_charge_inner(mut cgas: RegMut<CGAS>, mut ggas: RegMut<GGAS>, gas: Word) -> Result<(), RuntimeError> {
    if gas > *cgas {
        *ggas = arith::sub_word(*ggas, *cgas)?;
        *cgas = 0;

        Err(PanicReason::OutOfGas.into())
    } else {
        *cgas = arith::sub_word(*cgas, gas)?;
        *ggas = arith::sub_word(*ggas, gas)?;

        Ok(())
    }
}

#[allow(dead_code)]
pub(crate) struct ProfileGas<'a> {
    pub pc: Reg<'a, PC>,
    pub is: Reg<'a, IS>,
    pub current_contract: Option<ContractId>,
    pub profiler: &'a mut dyn ProfileCapture,
}

impl<'a> ProfileGas<'a> {
    #[allow(unused_variables)]
    pub(crate) fn profile(&mut self, cgas: Reg<CGAS>, gas: Word) {
        #[cfg(feature = "profile-coverage")]
        {
            let location = super::current_location(self.current_contract, self.pc, self.is);
            self.profiler.set_coverage(location);
        }

        #[cfg(feature = "profile-gas")]
        {
            let gas_use = gas.min(*cgas);
            let location = super::current_location(self.current_contract, self.pc, self.is);
            self.profiler.add_gas(location, gas_use);
        }
    }
}

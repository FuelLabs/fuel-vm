use super::Interpreter;
use crate::arith;
use crate::consts::*;
use crate::error::RuntimeError;
use crate::gas::DependentCost;

use fuel_asm::{Opcode, PanicReason};
use fuel_types::Word;

impl<S, Tx> Interpreter<S, Tx> {
    pub(crate) fn remaining_gas(&self) -> Word {
        self.registers[REG_GGAS]
    }

    /// Sets the remaining amout of gas to both CGAS and GGAS.
    /// Only useful in contexts where CGAS and GGAS are the same,
    /// i.e. predicates and testing.
    pub(crate) fn set_remaining_gas(&mut self, gas: Word) {
        self.registers[REG_GGAS] = gas;
        self.registers[REG_CGAS] = gas;
    }

    pub(crate) fn dependent_gas_charge(&mut self, gas_cost: DependentCost, arg: Word) -> Result<(), RuntimeError> {
        if gas_cost.dep_per_unit == 0 {
            self.gas_charge(gas_cost.base)
        } else {
            self.gas_charge(gas_cost.base.saturating_add(arg.saturating_div(gas_cost.dep_per_unit)))
        }
    }

    pub(crate) fn gas_charge(&mut self, gas: Word) -> Result<(), RuntimeError> {
        #[cfg(feature = "profile-coverage")]
        {
            let location = self.current_location();
            self.profiler.data_mut().coverage_mut().set(location);
        }

        #[cfg(feature = "profile-gas")]
        {
            let gas_use = gas.min(self.registers[REG_CGAS]);
            let location = self.current_location();
            self.profiler.data_mut().gas_mut().add(location, gas_use);
        }

        if gas > self.registers[REG_CGAS] {
            self.registers[REG_GGAS] = arith::sub_word(self.registers[REG_GGAS], self.registers[REG_CGAS])?;
            self.registers[REG_CGAS] = 0;

            Err(PanicReason::OutOfGas.into())
        } else {
            self.registers[REG_CGAS] = arith::sub_word(self.registers[REG_CGAS], gas)?;
            self.registers[REG_GGAS] = arith::sub_word(self.registers[REG_GGAS], gas)?;

            Ok(())
        }
    }
}

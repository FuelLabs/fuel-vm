use super::Interpreter;
use crate::error::SimpleResult;

use fuel_asm::{
    PanicReason,
    RegId,
};
use fuel_tx::DependentCost;
use fuel_types::Word;

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V> {
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
        let cost = gas_cost.resolve(arg);
        self.gas_charge(cost)
    }

    pub(crate) fn dependent_gas_charge_without_base(
        &mut self,
        gas_cost: DependentCost,
        arg: Word,
    ) -> SimpleResult<()> {
        let cost = gas_cost.resolve_without_base(arg);
        self.gas_charge(cost)
    }

    /// Do a gas charge with the given amount, panicing when running out of gas.
    /// Relies on the invariant that CGAS <= GGAS.
    pub fn gas_charge(&mut self, gas: Word) -> SimpleResult<()> {
        let cgas = self.registers[RegId::CGAS];
        let ggas = self.registers[RegId::GGAS];

        #[allow(clippy::arithmetic_side_effects)] // Safety: checked in if condition
        if gas > cgas {
            self.registers[RegId::GGAS] = ggas.saturating_sub(cgas);
            self.registers[RegId::CGAS] = 0;
            Err(PanicReason::OutOfGas.into())
        } else {
            // Happy path: gas <= cgas <= ggas, subtraction cannot underflow
            self.registers[RegId::CGAS] = cgas - gas;
            self.registers[RegId::GGAS] = ggas - gas;
            Ok(())
        }
    }
}

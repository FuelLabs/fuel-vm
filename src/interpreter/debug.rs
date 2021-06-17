use super::{Interpreter, ProgramState};
use crate::consts::*;
use crate::debug::DebugEval;

use fuel_asm::Word;
use fuel_tx::ContractAddress;

impl<S> Interpreter<S> {
    pub fn set_breakpoint(&mut self, contract: Option<ContractAddress>, pc: Word) {
        let contract = contract.unwrap_or_default();

        self.debugger.set_breakpoint(contract, pc)
    }

    pub fn remove_breakpoint(&mut self, contract: Option<ContractAddress>, pc: Word) {
        let contract = contract.unwrap_or_default();

        self.debugger.remove_breakpoint(&contract, pc)
    }

    pub fn eval_debugger_state(&mut self) -> DebugEval {
        let debugger = &mut self.debugger;

        let contract = self.frames.last().map(|f| f.to()).copied();
        let pc = match &contract {
            Some(_) => self.registers[REG_PC].saturating_sub(self.registers[REG_IS]),
            None => self.registers[REG_PC],
        };

        // Default contract address maps to unset contract target
        let contract = contract.unwrap_or_default();

        debugger.eval_state(contract, pc)
    }

    pub fn debugger_set_last_state(&mut self, state: ProgramState) {
        self.debugger.set_last_state(state)
    }

    pub const fn debugger_last_state(&self) -> &Option<ProgramState> {
        self.debugger.last_state()
    }
}

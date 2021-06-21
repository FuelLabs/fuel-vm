use super::{CallFrame, Interpreter, ProgramState};
use crate::consts::*;
use crate::debug::{Breakpoint, DebugEval};

impl<S> Interpreter<S> {
    pub fn set_breakpoint(&mut self, breakpoint: Breakpoint) {
        self.debugger.set_breakpoint(breakpoint)
    }

    pub fn remove_breakpoint(&mut self, breakpoint: &Breakpoint) {
        self.debugger.remove_breakpoint(breakpoint)
    }

    pub fn eval_debugger_state(&mut self) -> DebugEval {
        let debugger = &mut self.debugger;

        let contract = self.frames.last().map(CallFrame::to);
        let pc = self.registers[REG_PC].saturating_sub(self.registers[REG_IS]);

        debugger.eval_state(contract, pc)
    }

    pub fn debugger_set_last_state(&mut self, state: ProgramState) {
        self.debugger.set_last_state(state)
    }

    pub const fn debugger_last_state(&self) -> &Option<ProgramState> {
        self.debugger.last_state()
    }
}

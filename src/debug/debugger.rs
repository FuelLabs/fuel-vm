use crate::interpreter::ProgramState;

use fuel_asm::Word;
use fuel_tx::ContractAddress;

use std::collections::{HashMap, HashSet};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Breakpoint {
    contract: ContractAddress,
    pc: Word,
}

impl Breakpoint {
    pub const fn new(contract: ContractAddress, pc: Word) -> Self {
        Self { contract, pc }
    }

    pub const fn contract(&self) -> &ContractAddress {
        &self.contract
    }

    pub const fn pc(&self) -> Word {
        self.pc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DebugEval {
    Breakpoint(Breakpoint),
    Continue,
}

impl Default for DebugEval {
    fn default() -> Self {
        Self::Continue
    }
}

impl From<Breakpoint> for DebugEval {
    fn from(b: Breakpoint) -> Self {
        Self::Breakpoint(b)
    }
}

impl DebugEval {
    pub const fn should_continue(&self) -> bool {
        match self {
            Self::Continue => true,
            _ => false,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Debugger {
    breakpoints: HashMap<ContractAddress, HashSet<Word>>,
    last_state: Option<ProgramState>,
}

impl Debugger {
    pub fn set_breakpoint(&mut self, contract: ContractAddress, pc: Word) {
        self.breakpoints
            .get_mut(&contract)
            .map(|set| set.insert(pc))
            .map(|_| ())
            .unwrap_or_else(|| {
                let mut set = HashSet::new();

                set.insert(pc);

                self.breakpoints.insert(contract, set);
            });
    }

    pub fn remove_breakpoint(&mut self, contract: &ContractAddress, pc: Word) {
        self.breakpoints.get_mut(contract).map(|set| set.remove(&pc));
    }

    pub fn eval_state(&mut self, contract: ContractAddress, pc: Word) -> DebugEval {
        let last_state = self.last_state.take();

        self.breakpoints
            .get(&contract)
            .map(|set| set.get(&pc))
            .flatten()
            .map(|_| {
                let breakpoint = Breakpoint::new(contract, pc);

                match last_state {
                    Some(s) if s == breakpoint => DebugEval::Continue,
                    _ => breakpoint.into(),
                }
            })
            .unwrap_or_default()
    }

    pub fn set_last_state(&mut self, state: ProgramState) {
        self.last_state.replace(state);
    }

    pub const fn last_state(&self) -> &Option<ProgramState> {
        &self.last_state
    }
}

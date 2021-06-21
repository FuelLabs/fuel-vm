use crate::interpreter::ProgramState;

use fuel_asm::{Opcode, Word};
use fuel_tx::ContractAddress;

use std::collections::{HashMap, HashSet};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Breakpoint {
    contract: ContractAddress,
    pc: Word,
}

impl Breakpoint {
    const fn raw(contract: ContractAddress, pc: Word) -> Self {
        Self { contract, pc }
    }

    /// Create a new contract breakpoint
    ///
    /// The `$pc` is provided in op count and internally is multiplied by the op
    /// size. Also, the op count is always relative to `$is` so it should
    /// consider only the bytecode of the contract.
    pub const fn new(contract: ContractAddress, pc: Word) -> Self {
        let pc = pc * (Opcode::BYTES_SIZE as Word);

        Self::raw(contract, pc)
    }

    /// Create a new script breakpoint
    ///
    /// The `$pc` is provided in op count and internally is multiplied by the op
    /// size
    pub fn script(pc: Word) -> Self {
        let contract = Default::default();

        Self::new(contract, pc)
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

    pub const fn breakpoint(&self) -> Option<&Breakpoint> {
        match self {
            Self::Breakpoint(b) => Some(b),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Debugger {
    breakpoints: HashMap<ContractAddress, HashSet<Word>>,
    last_state: Option<ProgramState>,
}

impl Debugger {
    pub fn set_breakpoint(&mut self, breakpoint: Breakpoint) {
        let contract = *breakpoint.contract();
        let pc = breakpoint.pc();

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

    pub fn remove_breakpoint(&mut self, breakpoint: &Breakpoint) {
        self.breakpoints
            .get_mut(breakpoint.contract())
            .map(|set| set.remove(&breakpoint.pc()));
    }

    pub fn eval_state(&mut self, contract: Option<&ContractAddress>, pc: Word) -> DebugEval {
        // Default contract address maps to unset contract target
        let contract = contract.copied().unwrap_or_default();
        let last_state = self.last_state.take();

        self.breakpoints
            .get(&contract)
            .map(|set| set.get(&pc))
            .flatten()
            .map(|_| {
                let breakpoint = Breakpoint::raw(contract, pc);

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

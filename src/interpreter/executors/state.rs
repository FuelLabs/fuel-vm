use crate::interpreter::LogEvent;

use fuel_asm::Word;
use fuel_tx::Transaction;

#[cfg(feature = "debug")]
use crate::debug::{Breakpoint, DebugEval};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecuteState {
    Proceed,
    Return(Word),

    #[cfg(feature = "debug")]
    DebugEvent(DebugEval),
}

impl Default for ExecuteState {
    fn default() -> Self {
        Self::Proceed
    }
}

#[cfg(feature = "debug")]
impl From<DebugEval> for ExecuteState {
    fn from(d: DebugEval) -> Self {
        Self::DebugEvent(d)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
pub enum ProgramState {
    Return(Word),

    #[cfg(feature = "debug")]
    RunProgram(DebugEval),

    #[cfg(feature = "debug")]
    VerifyPredicate(DebugEval),
}

#[cfg(feature = "debug")]
impl PartialEq<Breakpoint> for ProgramState {
    fn eq(&self, other: &Breakpoint) -> bool {
        match self.debug_ref() {
            Some(&DebugEval::Breakpoint(b)) => &b == other,
            _ => false,
        }
    }
}

#[cfg(feature = "debug")]
impl ProgramState {
    pub const fn debug_ref(&self) -> Option<&DebugEval> {
        match self {
            Self::RunProgram(d) | Self::VerifyPredicate(d) => Some(d),
            _ => None,
        }
    }

    pub const fn is_debug(&self) -> bool {
        self.debug_ref().is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransition {
    state: ProgramState,
    tx: Transaction,
    log: Vec<LogEvent>,
}

impl StateTransition {
    pub const fn new(state: ProgramState, tx: Transaction, log: Vec<LogEvent>) -> Self {
        Self { state, tx, log }
    }

    pub const fn state(&self) -> &ProgramState {
        &self.state
    }

    pub const fn tx(&self) -> &Transaction {
        &self.tx
    }

    pub fn log(&self) -> &[LogEvent] {
        self.log.as_slice()
    }

    pub fn into_inner(self) -> (ProgramState, Transaction, Vec<LogEvent>) {
        (self.state, self.tx, self.log)
    }
}

impl From<StateTransition> for ProgramState {
    fn from(t: StateTransition) -> ProgramState {
        t.state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateTransitionRef<'a> {
    state: ProgramState,
    tx: &'a Transaction,
    log: &'a [LogEvent],
}

impl<'a> StateTransitionRef<'a> {
    pub const fn new(state: ProgramState, tx: &'a Transaction, log: &'a [LogEvent]) -> Self {
        Self { state, tx, log }
    }

    pub const fn state(&self) -> &ProgramState {
        &self.state
    }

    pub const fn tx(&self) -> &Transaction {
        self.tx
    }

    pub const fn log(&self) -> &[LogEvent] {
        self.log
    }

    pub fn into_owned(self) -> StateTransition {
        StateTransition::new(self.state, self.tx.clone(), self.log.to_vec())
    }
}

impl<'a> From<&'a StateTransition> for StateTransitionRef<'a> {
    fn from(t: &'a StateTransition) -> StateTransitionRef<'a> {
        Self {
            state: *t.state(),
            tx: t.tx(),
            log: t.log(),
        }
    }
}

impl<'a> From<StateTransitionRef<'a>> for ProgramState {
    fn from(t: StateTransitionRef<'a>) -> ProgramState {
        t.state
    }
}

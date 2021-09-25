use fuel_data::{Bytes32, Word};
use fuel_tx::{Receipt, Transaction};

#[cfg(feature = "debug")]
mod debug;

#[cfg(feature = "debug")]
mod debugger;

#[cfg(feature = "debug")]
pub use debug::{Breakpoint, DebugEval};

#[cfg(feature = "debug")]
pub use debugger::Debugger;

#[cfg(not(feature = "debug"))]
pub type Debugger = ();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecuteState {
    Proceed,
    Return(Word),
    ReturnData(Bytes32),

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
    ReturnData(Bytes32),

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
    receipts: Vec<Receipt>,
}

impl StateTransition {
    pub const fn new(state: ProgramState, tx: Transaction, receipts: Vec<Receipt>) -> Self {
        Self { state, tx, receipts }
    }

    pub const fn state(&self) -> &ProgramState {
        &self.state
    }

    pub const fn tx(&self) -> &Transaction {
        &self.tx
    }

    pub fn receipts(&self) -> &[Receipt] {
        self.receipts.as_slice()
    }

    pub fn into_inner(self) -> (ProgramState, Transaction, Vec<Receipt>) {
        (self.state, self.tx, self.receipts)
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
    receipts: &'a [Receipt],
}

impl<'a> StateTransitionRef<'a> {
    pub const fn new(state: ProgramState, tx: &'a Transaction, receipts: &'a [Receipt]) -> Self {
        Self { state, tx, receipts }
    }

    pub const fn state(&self) -> &ProgramState {
        &self.state
    }

    pub const fn tx(&self) -> &Transaction {
        self.tx
    }

    pub const fn receipts(&self) -> &[Receipt] {
        self.receipts
    }

    pub fn into_owned(self) -> StateTransition {
        StateTransition::new(self.state, self.tx.clone(), self.receipts.to_vec())
    }
}

impl<'a> From<&'a StateTransition> for StateTransitionRef<'a> {
    fn from(t: &'a StateTransition) -> StateTransitionRef<'a> {
        Self {
            state: *t.state(),
            tx: t.tx(),
            receipts: t.receipts(),
        }
    }
}

impl<'a> From<StateTransitionRef<'a>> for ProgramState {
    fn from(t: StateTransitionRef<'a>) -> ProgramState {
        t.state
    }
}

//! Runtime state representation for the VM

use alloc::vec::Vec;

use fuel_tx::Receipt;
use fuel_types::{
    Bytes32,
    Word,
};

mod debug;

mod debugger;

pub use debug::{
    Breakpoint,
    DebugEval,
};

pub use debugger::Debugger;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Resulting state of an instruction set execution.
pub enum ExecuteState {
    /// The VM should proceed normally with the execution.
    Proceed,
    /// The current context returned a [`Word`].
    Return(Word),
    /// The current context returned some data represented as its digest.
    ReturnData(Bytes32),
    /// The set execution resulted in a `RVRT` instruction.
    Revert(Word),

    /// A debug event was reached.
    DebugEvent(DebugEval),
}

impl ExecuteState {
    /// Return true if the VM execution should continue.
    pub const fn should_continue(&self) -> bool {
        matches!(self, Self::Proceed | Self::DebugEvent(DebugEval::Continue))
    }
}

impl Default for ExecuteState {
    fn default() -> Self {
        Self::Proceed
    }
}

impl From<DebugEval> for ExecuteState {
    fn from(d: DebugEval) -> Self {
        Self::DebugEvent(d)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Resulting state of a transaction/program execution.
pub enum ProgramState {
    /// The transaction returned a [`Word`].
    Return(Word),
    /// The transaction returned some data represented as its digest.
    ReturnData(Bytes32),
    /// The transaction execution resulted in a `RVRT` instruction.
    Revert(Word),

    /// A debug event was reached for the transaction. The VM is suspended.
    RunProgram(DebugEval),

    /// A debug event was reached for a predicate verification. The VM is
    /// suspended.
    VerifyPredicate(DebugEval),
}

impl PartialEq<Breakpoint> for ProgramState {
    fn eq(&self, other: &Breakpoint) -> bool {
        match self.debug_ref() {
            Some(&DebugEval::Breakpoint(b)) => &b == other,
            _ => false,
        }
    }
}

impl ProgramState {
    /// Debug event representation.
    ///
    /// Will return `None` if no debug event was reached.
    pub const fn debug_ref(&self) -> Option<&DebugEval> {
        match self {
            Self::RunProgram(d) | Self::VerifyPredicate(d) => Some(d),
            _ => None,
        }
    }

    /// Return `true` if a debug event was reached.
    pub const fn is_debug(&self) -> bool {
        self.debug_ref().is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Representation of the result of a transaction execution.
pub struct StateTransition<Tx> {
    state: ProgramState,
    tx: Tx,
    receipts: Vec<Receipt>,
}

impl<Tx> StateTransition<Tx> {
    /// Create a new state transition representation.
    pub const fn new(state: ProgramState, tx: Tx, receipts: Vec<Receipt>) -> Self {
        Self {
            state,
            tx,
            receipts,
        }
    }

    /// Program state representation.
    pub const fn state(&self) -> &ProgramState {
        &self.state
    }

    /// Resulting mutated transaction after VM execution.
    pub const fn tx(&self) -> &Tx {
        &self.tx
    }

    /// Flag whether the client should revert after execution.
    pub fn should_revert(&self) -> bool {
        self.receipts
            .iter()
            .any(|r| matches!(r, Receipt::Revert { .. } | Receipt::Panic { .. }))
    }

    /// Transaction receipts representing the state transition.
    pub fn receipts(&self) -> &[Receipt] {
        self.receipts.as_slice()
    }

    /// Convert this instance into its internal attributes.
    pub fn into_inner(self) -> (ProgramState, Tx, Vec<Receipt>) {
        (self.state, self.tx, self.receipts)
    }
}

impl<Tx> From<StateTransition<Tx>> for ProgramState {
    fn from(t: StateTransition<Tx>) -> ProgramState {
        t.state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Zero-copy Representation of the result of a transaction execution bound to
/// the lifetime of the VM.
pub struct StateTransitionRef<'a, Tx> {
    state: ProgramState,
    tx: &'a Tx,
    receipts: &'a [Receipt],
}

impl<'a, Tx> StateTransitionRef<'a, Tx> {
    /// Create a new by reference state transition representation.
    pub const fn new(state: ProgramState, tx: &'a Tx, receipts: &'a [Receipt]) -> Self {
        Self {
            state,
            tx,
            receipts,
        }
    }

    /// Program state representation.
    pub const fn state(&self) -> &ProgramState {
        &self.state
    }

    /// Resulting mutated transaction after VM execution.
    pub const fn tx(&self) -> &Tx {
        self.tx
    }

    /// Transaction receipts representing the state transition.
    pub const fn receipts(&self) -> &[Receipt] {
        self.receipts
    }

    /// Flag whether the client should revert after execution.
    pub fn should_revert(&self) -> bool {
        self.receipts
            .iter()
            .any(|r| matches!(r, Receipt::Revert { .. } | Receipt::Panic { .. }))
    }
}

impl<'a, Tx> From<&'a StateTransition<Tx>> for StateTransitionRef<'a, Tx> {
    fn from(t: &'a StateTransition<Tx>) -> StateTransitionRef<'a, Tx> {
        Self {
            state: *t.state(),
            tx: t.tx(),
            receipts: t.receipts(),
        }
    }
}

impl<Tx: Clone> From<StateTransitionRef<'_, Tx>> for StateTransition<Tx> {
    fn from(t: StateTransitionRef<Tx>) -> StateTransition<Tx> {
        StateTransition {
            state: *t.state(),
            tx: t.tx().clone(),
            receipts: t.receipts().to_vec(),
        }
    }
}

impl<'a, Tx: Clone> From<StateTransitionRef<'a, Tx>> for ProgramState {
    fn from(t: StateTransitionRef<'a, Tx>) -> ProgramState {
        t.state
    }
}

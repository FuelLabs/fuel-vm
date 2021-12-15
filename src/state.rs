//! Runtime state representation for the VM

use fuel_tx::{Receipt, Transaction};
use fuel_types::{Bytes32, Word};

#[cfg(feature = "debug")]
mod debug;

#[cfg(feature = "debug")]
mod debugger;

#[cfg(feature = "debug")]
pub use debug::{Breakpoint, DebugEval};

#[cfg(feature = "debug")]
pub use debugger::Debugger;

#[cfg(not(feature = "debug"))]
/// Fallback functionless implementation if `debug` feature isn't enabled.
pub type Debugger = ();

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

    #[cfg(feature = "debug")]
    /// A debug event was reached.
    DebugEvent(DebugEval),
}

impl ExecuteState {
    /// Return true if the VM execution should continue.
    pub const fn should_continue(&self) -> bool {
        #[cfg(not(feature = "debug"))]
        {
            matches!(self, Self::Proceed)
        }

        #[cfg(feature = "debug")]
        {
            matches!(self, Self::Proceed | Self::DebugEvent(DebugEval::Continue))
        }
    }
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
/// Resulting state of a transaction/program execution.
pub enum ProgramState {
    /// The transaction returned a [`Word`].
    Return(Word),
    /// The transaction returned some data represented as its digest.
    ReturnData(Bytes32),
    /// The transaction execution resulted in a `RVRT` instruction.
    Revert(Word),

    #[cfg(feature = "debug")]
    /// A debug event was reached for the transaction. The VM is suspended.
    RunProgram(DebugEval),

    #[cfg(feature = "debug")]
    /// A debug event was reached for a predicate verification. The VM is
    /// suspended.
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
pub struct StateTransition {
    state: ProgramState,
    tx: Transaction,
    receipts: Vec<Receipt>,
}

impl StateTransition {
    /// Create a new state transition representation.
    pub const fn new(state: ProgramState, tx: Transaction, receipts: Vec<Receipt>) -> Self {
        Self { state, tx, receipts }
    }

    /// Program state representation.
    pub const fn state(&self) -> &ProgramState {
        &self.state
    }

    /// Resulting mutated transaction after VM execution.
    pub const fn tx(&self) -> &Transaction {
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
/// Zero-copy Representation of the result of a transaction execution bound to
/// the lifetime of the VM.
pub struct StateTransitionRef<'a> {
    state: ProgramState,
    tx: &'a Transaction,
    receipts: &'a [Receipt],
}

impl<'a> StateTransitionRef<'a> {
    /// Create a new by reference state transition representation.
    pub const fn new(state: ProgramState, tx: &'a Transaction, receipts: &'a [Receipt]) -> Self {
        Self { state, tx, receipts }
    }

    /// Program state representation.
    pub const fn state(&self) -> &ProgramState {
        &self.state
    }

    /// Resulting mutated transaction after VM execution.
    pub const fn tx(&self) -> &Transaction {
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

    /// Convert this instance into an owned state transition, cloning its
    /// internals.
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

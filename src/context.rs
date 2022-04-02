//! VM runtime context definitions

use crate::interpreter::MemoryRange;
use fuel_tx::Transaction;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
/// Runtime context description.
pub enum Context {
    /// Current context is a predicate verification.
    Predicate { range: MemoryRange },
    /// Current context is a script execution.
    Script,
    /// Current context is under a `CALL` scop.e
    Call,
    /// Current context is a create transaction.
    /// Only used for contract setup and post execution steps (no bytecode execution).
    Create,
    /// No transaction initialized/invalid context.
    NotInitialized,
}

impl Default for Context {
    fn default() -> Self {
        Self::NotInitialized
    }
}

impl Context {
    /// Return `true` if the context is external; `false` otherwise.
    pub const fn is_external(&self) -> bool {
        matches!(self, Self::Predicate | Self::Script)
    }
}

impl From<&Transaction> for Context {
    fn from(tx: &Transaction) -> Self {
        match tx {
            Transaction::Script { .. } => Context::Script,
            Transaction::Create { .. } => Context::Create,
        }
    }
}

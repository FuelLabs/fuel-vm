//! VM runtime context definitions

use fuel_tx::Transaction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Runtime context description.
pub enum Context {
    /// Current context is a predicate verification.
    Predicate,
    /// Current context is a script execution.
    Script,
    /// Current context is under a `CALL` scop.e
    Call,
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
        if tx.is_script() {
            Self::Script
        } else {
            Self::Predicate
        }
    }
}

//! VM runtime context definitions

use crate::predicate::RuntimePredicate;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Runtime context description.
pub enum Context {
    /// Current context is a predicate verification.
    Predicate {
        /// Predicate program to be executed
        program: RuntimePredicate,
    },
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
        matches!(self, Self::Predicate { .. } | Self::Script)
    }

    /// Return the program to be executed, if its a predicate
    pub const fn predicate(&self) -> Option<&RuntimePredicate> {
        match self {
            Context::Predicate { program } => Some(program),
            _ => None,
        }
    }
}

//! VM runtime context definitions

use fuel_tx::Transaction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
pub enum Context {
    Predicate,
    Script,
    Call,
    NotInitialized,
}

impl Default for Context {
    fn default() -> Self {
        Self::NotInitialized
    }
}

impl Context {
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

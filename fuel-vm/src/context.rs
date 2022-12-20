//! VM runtime context definitions

use crate::predicate::RuntimePredicate;

use fuel_asm::Word;

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
    Script {
        /// Block height of the context
        block_height: u32,
    },
    /// Current context is under a `CALL` scope
    Call {
        /// Block height of the context
        block_height: u32,
    },
    /// No transaction initialized/invalid context.
    NotInitialized,
}

impl Default for Context {
    fn default() -> Self {
        Self::NotInitialized
    }
}

impl Context {
    /// Check if the context is predicate
    pub const fn is_predicate(&self) -> bool {
        matches!(self, Self::Predicate { .. })
    }

    /// Return `true` if the context is external; `false` otherwise.
    pub const fn is_external(&self) -> bool {
        matches!(self, Self::Predicate { .. } | Self::Script { .. })
    }

    /// Return the program to be executed, if its a predicate
    pub const fn predicate(&self) -> Option<&RuntimePredicate> {
        match self {
            Context::Predicate { program } => Some(program),
            _ => None,
        }
    }

    /// Return the block height from the context, if either script or call
    pub const fn block_height(&self) -> Option<u32> {
        match self {
            Context::Script { block_height } | Context::Call { block_height } => Some(*block_height),
            _ => None,
        }
    }

    /// Update the context according to the provided frame pointer
    pub fn update_from_frame_pointer(&mut self, fp: Word) {
        match self {
            Context::Script { block_height } if fp != 0 => {
                *self = Self::Call {
                    block_height: *block_height,
                }
            }

            Context::Call { block_height } if fp == 0 => {
                *self = Self::Script {
                    block_height: *block_height,
                }
            }
            _ => (),
        }
    }
}

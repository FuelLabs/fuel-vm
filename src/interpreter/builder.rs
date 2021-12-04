//! Builder pattern implementation for [`Interpreter`]
//!
//! Based on <https://doc.rust-lang.org/1.5.0/style/ownership/builders.html#non-consuming-builders-preferred>
//!
//! Follows the recommended `Non-consuming builder`.

use super::Interpreter;
use crate::client::MemoryStorage;
use crate::consts::*;
use crate::context::Context;
use crate::state::Debugger;

use fuel_tx::Transaction;

impl<S> Interpreter<S> {
    pub fn with_storage(storage: S) -> Self {
        Self {
            registers: [0; VM_REGISTER_COUNT],
            memory: vec![0; VM_MAX_RAM as usize],
            frames: vec![],
            receipts: vec![],
            tx: Transaction::default(),
            storage,
            debugger: Debugger::default(),
            context: Context::default(),
            block_height: 0,
        }
    }
}

impl<S> Default for Interpreter<S>
where
    S: Default,
{
    fn default() -> Self {
        Self::with_storage(Default::default())
    }
}

impl Interpreter<()> {
    pub fn without_storage() -> Self {
        Self::default()
    }
}

impl Interpreter<MemoryStorage> {
    pub fn with_memory_storage() -> Self {
        Self::default()
    }
}

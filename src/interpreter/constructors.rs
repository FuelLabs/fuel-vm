//! Exposed constructors API for the [`Interpreter`]

use fuel_tx::Transaction;

use super::Interpreter;
use crate::consts::*;
use crate::context::Context;
use crate::prelude::*;
use crate::state::Debugger;

#[cfg(feature = "profile-any")]
use crate::profiler::{ProfileReceiver, Profiler};

impl<S> Interpreter<S> {
    /// Create a new interpreter instance out of a storage implementation.
    ///
    /// If the provided storage implements
    /// [`crate::storage::InterpreterStorage`], the returned interpreter
    /// will provide full functionality.
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
            #[cfg(feature = "profile-any")]
            profiler: Profiler::default(),
            unused_balance_index: Default::default(),
        }
    }

    /// Sets a profiler for the VM
    #[cfg(feature = "profile-any")]
    pub fn with_profiling(mut self, receiver: Box<dyn ProfileReceiver>) -> Self {
        self.profiler.set_receiver(receiver);
        self
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
    /// Create a new interpreter without a storage backend.
    ///
    /// It will have restricted capabilities.
    pub fn without_storage() -> Self {
        Self::default()
    }
}

impl Interpreter<MemoryStorage> {
    /// Create a new storage with a provided in-memory storage.
    ///
    /// It will have full capabilities.
    pub fn with_memory_storage() -> Self {
        Self::default()
    }
}

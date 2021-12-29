//! Exposed constructors API for the [`Interpreter`]

use super::Interpreter;
use crate::consts::*;
use crate::context::Context;
use crate::memory_client::MemoryStorage;

#[cfg(feature = "debug")]
use crate::debug::Debugger;

#[cfg(feature = "profiler-any")]
use crate::profiler::{ProfileReceiver, Profiler};

use fuel_tx::Transaction;

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
            context: Context::default(),
            block_height: 0,

            #[cfg(feature = "debug")]
            debugger: Debugger::default(),

            #[cfg(feature = "profiler-any")]
            profiler: Profiler::default(),
        }
    }

    /// Sets a profiler for the VM
    #[cfg(feature = "profiler-any")]
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

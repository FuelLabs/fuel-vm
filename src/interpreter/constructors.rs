//! Exposed constructors API for the [`Interpreter`]

use super::{ExecutableTransaction, Interpreter, RuntimeBalances};
use crate::consts::*;
use crate::context::Context;
use crate::state::Debugger;
use crate::storage::MemoryStorage;

#[cfg(feature = "profile-any")]
use crate::profiler::{ProfileReceiver, Profiler};

use fuel_tx::ConsensusParameters;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    /// Create a new interpreter instance out of a storage implementation.
    ///
    /// If the provided storage implements
    /// [`crate::storage::InterpreterStorage`], the returned interpreter
    /// will provide full functionality.
    pub fn with_storage(storage: S, params: ConsensusParameters) -> Self {
        Self {
            registers: [0; VM_REGISTER_COUNT],
            memory: vec![0; VM_MAX_RAM as usize],
            frames: vec![],
            receipts: vec![],
            tx: Default::default(),
            initial_balances: Default::default(),
            storage,
            debugger: Debugger::default(),
            context: Context::default(),
            balances: RuntimeBalances::default(),
            #[cfg(feature = "profile-any")]
            profiler: Profiler::default(),
            params,
        }
    }

    /// Set the consensus parameters for the interpreter
    pub fn with_params(&mut self, params: ConsensusParameters) -> &mut Self {
        self.params = params;
        self
    }

    /// Sets a profiler for the VM
    #[cfg(feature = "profile-any")]
    pub fn with_profiler<P>(&mut self, receiver: P) -> &mut Self
    where
        P: ProfileReceiver + Send + Sync + 'static,
    {
        self.profiler.set_receiver(Box::new(receiver));
        self
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    S: Clone,
    Tx: ExecutableTransaction,
{
    /// Build the interpreter
    pub fn build(&mut self) -> Self {
        self.clone()
    }
}

impl<S, Tx> Default for Interpreter<S, Tx>
where
    S: Default,
    Tx: ExecutableTransaction,
{
    fn default() -> Self {
        Self::with_storage(Default::default(), Default::default())
    }
}

impl<Tx> Interpreter<(), Tx>
where
    Tx: ExecutableTransaction,
{
    /// Create a new interpreter without a storage backend.
    ///
    /// It will have restricted capabilities.
    pub fn without_storage() -> Self {
        Self::default()
    }
}

impl<Tx> Interpreter<MemoryStorage, Tx>
where
    Tx: ExecutableTransaction,
{
    /// Create a new storage with a provided in-memory storage.
    ///
    /// It will have full capabilities.
    pub fn with_memory_storage() -> Self {
        Self::default()
    }
}

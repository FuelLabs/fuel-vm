//! Exposed constructors API for the [`Interpreter`]
#![allow(clippy::default_constructed_unit_structs)] // need for ::default() depends on cfg

use super::{
    ExecutableTransaction,
    Interpreter,
    Memory,
    RuntimeBalances,
};
use crate::{
    consts::*,
    context::Context,
    interpreter::{
        InterpreterParams,
        PanicContext,
    },
    state::Debugger,
};

use alloc::vec;

#[cfg(feature = "profile-any")]
use crate::profiler::ProfileReceiver;

use crate::profiler::Profiler;

#[cfg(feature = "test-helpers")]
use crate::{
    interpreter::EcalHandler,
    storage::MemoryStorage,
};

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
where
    Tx: Default,
    Ecal: Default,
{
    /// Create a new interpreter instance out of a storage implementation.
    ///
    /// If the provided storage implements
    /// [`crate::storage::InterpreterStorage`], the returned interpreter
    /// will provide full functionality.
    pub fn with_storage(storage: S, interpreter_params: InterpreterParams) -> Self {
        Self::with_storage_and_ecal(storage, interpreter_params, Ecal::default())
    }
}

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
where
    Tx: Default,
{
    /// Create a new interpreter instance out of a storage implementation.
    ///
    /// If the provided storage implements
    /// [`crate::storage::InterpreterStorage`], the returned interpreter
    /// will provide full functionality.
    pub fn with_storage_and_ecal(
        storage: S,
        interpreter_params: InterpreterParams,
        ecal_state: Ecal,
    ) -> Self {
        Self {
            registers: [0; VM_REGISTER_COUNT],
            memory: Memory::new(),
            frames: vec![],
            receipts: Default::default(),
            tx: Default::default(),
            initial_balances: Default::default(),
            storage,
            debugger: Debugger::default(),
            context: Context::default(),
            balances: RuntimeBalances::default(),
            profiler: Profiler::default(),
            interpreter_params,
            panic_context: PanicContext::None,
            ecal_state,
        }
    }
}

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal> {
    /// Sets a profiler for the VM
    #[cfg(feature = "profile-any")]
    pub fn with_profiler<P>(&mut self, receiver: P) -> &mut Self
    where
        P: ProfileReceiver + Send + Sync + 'static,
    {
        self.profiler.set_receiver(alloc::boxed::Box::new(receiver));
        self
    }
}

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
where
    S: Clone,
    Tx: ExecutableTransaction,
    Ecal: Clone,
{
    /// Build the interpreter
    pub fn build(&mut self) -> Self {
        self.clone()
    }
}

#[cfg(feature = "test-helpers")]
impl<S, Tx, Ecal> Default for Interpreter<S, Tx, Ecal>
where
    S: Default,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
{
    fn default() -> Self {
        Interpreter::<S, Tx, Ecal>::with_storage(
            Default::default(),
            InterpreterParams::default(),
        )
    }
}

#[cfg(test)]
impl<Tx, Ecal> Interpreter<(), Tx, Ecal>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
{
    /// Create a new interpreter without a storage backend.
    ///
    /// It will have restricted capabilities.
    pub fn without_storage() -> Self {
        Self::default()
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx, Ecal> Interpreter<MemoryStorage, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
{
    /// Create a new storage with a provided in-memory storage.
    ///
    /// It will have full capabilities.
    pub fn with_memory_storage() -> Self {
        Self::default()
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx, Ecal> Interpreter<MemoryStorage, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    /// Create a new storage with a provided in-memory storage.
    ///
    /// It will have full capabilities.
    pub fn with_memory_storage_and_ecal(ecal: Ecal) -> Self {
        Interpreter::<MemoryStorage, Tx, Ecal>::with_storage_and_ecal(
            Default::default(),
            InterpreterParams::default(),
            ecal,
        )
    }
}

//! Exposed constructors API for the [`Interpreter`]
#![allow(clippy::default_constructed_unit_structs)] // need for ::default() depends on cfg

#[cfg(any(test, feature = "test-helpers"))]
use super::{
    trace::ExecutionTraceHooks,
    ExecutableTransaction,
    MemoryInstance,
};
use super::{
    Interpreter,
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

impl<M, S, Tx, Ecal, Trace> Interpreter<M, S, Tx, Ecal, Trace>
where
    Tx: Default,
    Ecal: Default,
    Trace: Default,
{
    /// Create a new interpreter instance out of a storage implementation.
    ///
    /// If the provided storage implements
    /// [`crate::storage::InterpreterStorage`], the returned interpreter
    /// will provide full functionality.
    pub fn with_storage(
        memory: M,
        storage: S,
        interpreter_params: InterpreterParams,
    ) -> Self {
        Self::with_storage_and_ecal(memory, storage, interpreter_params, Ecal::default())
    }
}

impl<M, S, Tx, Ecal, Trace> Interpreter<M, S, Tx, Ecal, Trace>
where
    Tx: Default,
    Trace: Default,
{
    /// Create a new interpreter instance out of a storage implementation.
    ///
    /// If the provided storage implements
    /// [`crate::storage::InterpreterStorage`], the returned interpreter
    /// will provide full functionality.
    pub fn with_storage_and_ecal(
        memory: M,
        storage: S,
        interpreter_params: InterpreterParams,
        ecal_state: Ecal,
    ) -> Self {
        Self {
            registers: [0; VM_REGISTER_COUNT],
            memory,
            frames: vec![],
            receipts: Default::default(),
            tx: Default::default(),
            input_contracts: Default::default(),
            input_contracts_index_to_output_index: Default::default(),
            initial_balances: Default::default(),
            storage,
            debugger: Debugger::default(),
            context: Context::default(),
            balances: RuntimeBalances::default(),
            profiler: Profiler::default(),
            trace: Trace::default(),
            interpreter_params,
            panic_context: PanicContext::None,
            ecal_state,
        }
    }
}

impl<M, S, Tx, Ecal, Trace> Interpreter<M, S, Tx, Ecal, Trace> {
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

#[cfg(any(test, feature = "test-helpers"))]
impl<S, Tx, Ecal, Trace> Default for Interpreter<MemoryInstance, S, Tx, Ecal, Trace>
where
    S: Default,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
    Trace: Default,
{
    fn default() -> Self {
        Interpreter::<_, S, Tx, Ecal, Trace>::with_storage(
            MemoryInstance::new(),
            Default::default(),
            InterpreterParams::default(),
        )
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl<Tx, Ecal, Trace> Interpreter<MemoryInstance, (), Tx, Ecal, Trace>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
    Trace: ExecutionTraceHooks + Default,
{
    /// Create a new interpreter without a storage backend.
    ///
    /// It will have restricted capabilities.
    pub fn without_storage() -> Self {
        Self::default()
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx, Ecal, Trace> Interpreter<MemoryInstance, MemoryStorage, Tx, Ecal, Trace>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
    Trace: ExecutionTraceHooks + Default,
{
    /// Create a new storage with a provided in-memory storage.
    ///
    /// It will have full capabilities.
    pub fn with_memory_storage() -> Self {
        Self::default()
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx, Ecal, Trace> Interpreter<MemoryInstance, MemoryStorage, Tx, Ecal, Trace>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
    Trace: ExecutionTraceHooks + Default,
{
    /// Create a new storage with a provided in-memory storage.
    ///
    /// It will have full capabilities.
    pub fn with_memory_storage_and_ecal(ecal: Ecal) -> Self {
        Interpreter::<_, MemoryStorage, Tx, Ecal, Trace>::with_storage_and_ecal(
            MemoryInstance::new(),
            Default::default(),
            InterpreterParams::default(),
            ecal,
        )
    }
}

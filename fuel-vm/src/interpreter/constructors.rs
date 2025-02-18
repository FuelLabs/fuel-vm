//! Exposed constructors API for the [`Interpreter`]
#![allow(clippy::default_constructed_unit_structs)] // need for ::default() depends on cfg

#[cfg(any(test, feature = "test-helpers"))]
use super::{
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

#[cfg(feature = "test-helpers")]
use crate::{
    interpreter::EcalHandler,
    storage::MemoryStorage,
};

impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal>
where
    Tx: Default,
    Ecal: Default,
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

impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal>
where
    Tx: Default,
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

            interpreter_params,
            panic_context: PanicContext::None,
            ecal_state,
        }
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl<S, Tx, Ecal> Default for Interpreter<MemoryInstance, S, Tx, Ecal>
where
    S: Default,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
{
    fn default() -> Self {
        Interpreter::<_, S, Tx, Ecal>::with_storage(
            MemoryInstance::new(),
            Default::default(),
            InterpreterParams::default(),
        )
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl<Tx, Ecal> Interpreter<MemoryInstance, (), Tx, Ecal>
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
impl<Tx, Ecal> Interpreter<MemoryInstance, MemoryStorage, Tx, Ecal>
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
impl<Tx, Ecal> Interpreter<MemoryInstance, MemoryStorage, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    /// Create a new storage with a provided in-memory storage.
    ///
    /// It will have full capabilities.
    pub fn with_memory_storage_and_ecal(ecal: Ecal) -> Self {
        Interpreter::<_, MemoryStorage, Tx, Ecal>::with_storage_and_ecal(
            MemoryInstance::new(),
            Default::default(),
            InterpreterParams::default(),
            ecal,
        )
    }
}

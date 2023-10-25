//! Exposed constructors API for the [`Interpreter`]
#![allow(clippy::default_constructed_unit_structs)] // need for ::default() depends on cfg

use super::{
    EcalHandler,
    ExecutableTransaction,
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
    storage::MemoryStorage,
};

use alloc::vec;

#[cfg(feature = "profile-any")]
use crate::profiler::ProfileReceiver;

use crate::profiler::Profiler;

impl<S, Ecal, Tx> Interpreter<S, Ecal, Tx>
where
    Tx: Default,
{
    /// Create a new interpreter instance out of a storage implementation.
    ///
    /// If the provided storage implements
    /// [`crate::storage::InterpreterStorage`], the returned interpreter
    /// will provide full functionality.
    pub fn with_storage(storage: S, interpreter_params: InterpreterParams) -> Self {
        Self {
            registers: [0; VM_REGISTER_COUNT],
            memory: vec![0; MEM_SIZE]
                .try_into()
                .expect("Failed to allocate memory"),
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
            _ecal_handler: core::marker::PhantomData::<Ecal>,
        }
    }
}

impl<S, Ecal, Tx> Interpreter<S, Ecal, Tx> {
    /// Sets a profiler for the VM
    #[cfg(feature = "profile-any")]
    pub fn with_profiler<P>(&mut self, receiver: P) -> &mut Self
    where
        P: ProfileReceiver + Send + Sync + 'static,
    {
        self.profiler.set_receiver(alloc::boxed::Box::new(receiver));
        self
    }

    /// Sets ECAL opcode handler on type level
    pub fn with_ecal<NewEcal>(self) -> Interpreter<S, NewEcal, Tx> {
        Interpreter {
            _ecal_handler: core::marker::PhantomData::<NewEcal>,
            registers: self.registers,
            memory: self.memory,
            frames: self.frames,
            receipts: self.receipts,
            tx: self.tx,
            initial_balances: self.initial_balances,
            storage: self.storage,
            debugger: self.debugger,
            context: self.context,
            balances: self.balances,
            profiler: self.profiler,
            interpreter_params: self.interpreter_params,
            panic_context: self.panic_context,
        }
    }
}

impl<S, Ecal, Tx> Interpreter<S, Ecal, Tx>
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

impl<S, Ecal, Tx> Default for Interpreter<S, Ecal, Tx>
where
    S: Default,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    fn default() -> Self {
        Interpreter::<S, Ecal, Tx>::with_storage(
            Default::default(),
            InterpreterParams::default(),
        )
    }
}

#[cfg(test)]
impl<Ecal, Tx> Interpreter<(), Ecal, Tx>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    /// Create a new interpreter without a storage backend.
    ///
    /// It will have restricted capabilities.
    pub fn without_storage() -> Self {
        Self::default()
    }
}

impl<Ecal, Tx> Interpreter<MemoryStorage, Ecal, Tx>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    /// Create a new storage with a provided in-memory storage.
    ///
    /// It will have full capabilities.
    pub fn with_memory_storage() -> Self {
        Self::default()
    }
}

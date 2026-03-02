//! Exposed constructors API for the [`Interpreter`]
#![allow(clippy::default_constructed_unit_structs)] // need for ::default() depends on cfg

#[cfg(any(test, feature = "test-helpers"))]
use super::MemoryInstance;
use super::{
    EcalHandler,
    ExecutableTransaction,
    Interpreter,
    Memory,
    RuntimeBalances,
};
use crate::{
    consts::*,
    context::Context,
    error::IoResult,
    interpreter::{
        InterpreterParams,
        PanicContext,
    },
    state::{
        Debugger,
        ExecuteState,
    },
};

#[cfg(feature = "test-helpers")]
use crate::storage::MemoryStorage;
use crate::{
    prelude::InterpreterStorage,
    verification::Verifier,
};
use alloc::vec;
use fuel_asm::Opcode;

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction + Default,
    Ecal: EcalHandler + Default,
    V: Verifier + Default,
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

type Handler<M, S, Tx, Ecal, V, DataError> =
    for<'a> fn(
        &'a mut Interpreter<M, S, Tx, Ecal, V>,
        [u8; 3],
    ) -> IoResult<ExecuteState, DataError>;

#[allow(clippy::type_complexity)]
pub const fn handlers<M, S, Tx, Ecal, V>()
-> [Option<Handler<M, S, Tx, Ecal, V, S::DataError>>; 256]
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
    V: Verifier,
{
    use crate::interpreter::executors::instruction::ExecuteOptimized;

    macro_rules! cast_handler {
        ($e:expr) => {{
            #[allow(clippy::as_conversions)]
            let handler = $e as Handler<M, S, Tx, Ecal, V, S::DataError>;
            handler
        }};
    }

    let mut array = [None; 256];

    array[Opcode::ADD as usize] = Some(cast_handler!(
        <fuel_asm::op::ADD as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::DIV as usize] = Some(cast_handler!(
        <fuel_asm::op::DIV as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::EQ as usize] = Some(cast_handler!(
        <fuel_asm::op::EQ as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::GT as usize] = Some(cast_handler!(
        <fuel_asm::op::GT as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::LT as usize] = Some(cast_handler!(
        <fuel_asm::op::LT as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::MOD as usize] = Some(cast_handler!(
        <fuel_asm::op::MOD as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::MOVE as usize] = Some(cast_handler!(
        <fuel_asm::op::MOVE as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::MUL as usize] = Some(cast_handler!(
        <fuel_asm::op::MUL as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::RET as usize] = Some(cast_handler!(
        <fuel_asm::op::RET as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::LOG as usize] = Some(cast_handler!(
        <fuel_asm::op::LOG as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::LB as usize] = Some(cast_handler!(
        <fuel_asm::op::LB as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::LQW as usize] = Some(cast_handler!(
        <fuel_asm::op::LQW as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::LHW as usize] = Some(cast_handler!(
        <fuel_asm::op::LHW as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::LW as usize] = Some(cast_handler!(
        <fuel_asm::op::LW as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::MOVI as usize] = Some(cast_handler!(
        <fuel_asm::op::MOVI as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::JMPF as usize] = Some(cast_handler!(
        <fuel_asm::op::JMPF as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::JMPB as usize] = Some(cast_handler!(
        <fuel_asm::op::JMPB as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::JNZF as usize] = Some(cast_handler!(
        <fuel_asm::op::JNZF as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array[Opcode::ADDI as usize] = Some(cast_handler!(
        <fuel_asm::op::ADDI as ExecuteOptimized<M, S, Tx, Ecal, V>>::execute_opt
    ));
    array
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
    V: Verifier + Default,
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
            verifier: Default::default(),
            owner_ptr: None,
        }
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl<S, Tx, Ecal, V> Default for Interpreter<MemoryInstance, S, Tx, Ecal, V>
where
    S: InterpreterStorage + Default,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
    V: Verifier + Default,
{
    fn default() -> Self {
        Interpreter::<_, S, Tx, Ecal, V>::with_storage(
            MemoryInstance::new(),
            Default::default(),
            InterpreterParams::default(),
        )
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl<Tx, Ecal, V> Interpreter<MemoryInstance, MemoryStorage, Tx, Ecal, V>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
    V: Verifier + Default,
{
    /// Create a new interpreter without a storage backend.
    ///
    /// It will have restricted capabilities.
    pub fn without_storage() -> Self {
        Self::default()
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx, Ecal, V> Interpreter<MemoryInstance, MemoryStorage, Tx, Ecal, V>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler + Default,
    V: Verifier + Default,
{
    /// Create a new storage with a provided in-memory storage.
    ///
    /// It will have full capabilities.
    pub fn with_memory_storage() -> Self {
        Self::default()
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx, Ecal, V> Interpreter<MemoryInstance, MemoryStorage, Tx, Ecal, V>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
    V: Verifier + Default,
{
    /// Create a new storage with a provided in-memory storage.
    ///
    /// It will have full capabilities.
    pub fn with_memory_storage_and_ecal(ecal: Ecal) -> Self {
        Interpreter::<_, MemoryStorage, Tx, Ecal, V>::with_storage_and_ecal(
            MemoryInstance::new(),
            Default::default(),
            InterpreterParams::default(),
            ecal,
        )
    }
}

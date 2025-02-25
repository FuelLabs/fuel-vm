//! Backtrace implementation to track script errors.
//!
//! As of the moment, doesn't support predicates.

use alloc::{
    borrow::ToOwned,
    vec::Vec,
};

use crate::{
    call::CallFrame,
    consts::*,
    interpreter::{
        InitialBalances,
        Interpreter,
    },
};
use educe::Educe;

use crate::interpreter::{
    Memory,
    MemoryInstance,
};
use fuel_tx::ScriptExecutionResult;
use fuel_types::{
    ContractId,
    Word,
};

#[derive(Educe)]
#[educe(Debug)]
/// Runtime description derived from a VM error.
pub struct Backtrace {
    call_stack: Vec<CallFrame>,
    contract: ContractId,
    registers: [Word; VM_REGISTER_COUNT],
    memory: MemoryInstance,
    result: ScriptExecutionResult,
    initial_balances: InitialBalances,
}

impl Backtrace {
    /// Create a backtrace from a vm instance and instruction result.
    ///
    /// This isn't copy-free and shouldn't be provided by default.
    pub fn from_vm_error<M, S, Tx, Ecal, OnVerifyError>(
        vm: &Interpreter<M, S, Tx, Ecal, OnVerifyError>,
        result: ScriptExecutionResult,
    ) -> Self
    where
        M: Memory,
    {
        let call_stack = vm.call_stack().to_owned();
        let contract = vm.internal_contract().unwrap_or_default();
        let memory = vm.memory().clone();
        let initial_balances = vm.initial_balances().clone();
        let mut registers = [0; VM_REGISTER_COUNT];

        registers.copy_from_slice(vm.registers());

        Self {
            call_stack,
            contract,
            registers,
            memory,
            result,
            initial_balances,
        }
    }

    /// Call stack of the VM when the error occurred.
    pub fn call_stack(&self) -> &[CallFrame] {
        self.call_stack.as_slice()
    }

    /// Last contract of the context when the error occurred.
    pub const fn contract(&self) -> &ContractId {
        &self.contract
    }

    /// Register set when the error occurred.
    pub const fn registers(&self) -> &[Word] {
        &self.registers
    }

    /// Memory of the VM when the error occurred.
    pub fn memory(&self) -> &MemoryInstance {
        &self.memory
    }

    /// [`ScriptExecutionResult`] of the error that caused this backtrace.
    pub const fn result(&self) -> &ScriptExecutionResult {
        &self.result
    }

    /// The initial balances.
    pub const fn initial_balances(&self) -> &InitialBalances {
        &self.initial_balances
    }

    /// Expose the internal attributes of the backtrace.
    pub fn into_inner(
        self,
    ) -> (
        Vec<CallFrame>,
        ContractId,
        [Word; VM_REGISTER_COUNT],
        MemoryInstance,
        ScriptExecutionResult,
        InitialBalances,
    ) {
        let Self {
            call_stack,
            contract,
            registers,
            memory,
            result,
            initial_balances,
        } = self;

        (
            call_stack,
            contract,
            registers,
            memory,
            result,
            initial_balances,
        )
    }
}

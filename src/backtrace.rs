//! Backtrace implementation to track script errors.
//!
//! As of the moment, doesn't support predicates.

use crate::call::CallFrame;
use crate::consts::*;
use crate::interpreter::{InitialBalances, Interpreter};

use fuel_tx::ScriptExecutionResult;
use fuel_types::{ContractId, Word};

#[derive(Debug)]
/// Runtime description derived from a VM error.
pub struct Backtrace {
    call_stack: Vec<CallFrame>,
    contract: ContractId,
    registers: [Word; VM_REGISTER_COUNT],
    memory: Vec<u8>,
    result: ScriptExecutionResult,
    initial_balances: InitialBalances,
}

impl Backtrace {
    /// Create a backtrace from a vm instance and instruction result.
    ///
    /// This isn't copy-free and shouldn't be provided by default.
    pub fn from_vm_error<S, Tx>(vm: &Interpreter<S, Tx>, result: ScriptExecutionResult) -> Self {
        let call_stack = vm.call_stack().to_owned();
        let contract = vm.internal_contract_or_default();
        let memory = vm.memory().to_owned();
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
    pub fn memory(&self) -> &[u8] {
        self.memory.as_slice()
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
        Vec<u8>,
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

        (call_stack, contract, registers, memory, result, initial_balances)
    }
}

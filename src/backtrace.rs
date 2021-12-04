//! Backtrace implementation to track script errors.
//!
//! As of the moment, doesn't support predicates.

use crate::call::CallFrame;
use crate::consts::*;
use crate::interpreter::Interpreter;

use fuel_asm::InstructionResult;
use fuel_types::{ContractId, Word};

#[derive(Debug)]
pub struct Backtrace {
    call_stack: Vec<CallFrame>,
    contract: ContractId,
    registers: [Word; VM_REGISTER_COUNT],
    memory: Vec<u8>,
    result: InstructionResult,
}

impl Backtrace {
    pub fn from_vm_error<S>(vm: &Interpreter<S>, result: InstructionResult) -> Self {
        let call_stack = vm.call_stack().to_owned();
        let contract = vm.internal_contract_or_default();
        let memory = vm.memory().to_owned();
        let mut registers = [0; VM_REGISTER_COUNT];

        registers.copy_from_slice(vm.registers());

        Self {
            call_stack,
            contract,
            registers,
            memory,
            result,
        }
    }

    pub fn call_stack(&self) -> &[CallFrame] {
        self.call_stack.as_slice()
    }

    pub const fn contract(&self) -> &ContractId {
        &self.contract
    }

    pub const fn registers(&self) -> &[Word] {
        &self.registers
    }

    pub fn memory(&self) -> &[u8] {
        self.memory.as_slice()
    }

    pub const fn result(&self) -> &InstructionResult {
        &self.result
    }

    pub fn into_inner(
        self,
    ) -> (
        Vec<CallFrame>,
        ContractId,
        [Word; VM_REGISTER_COUNT],
        Vec<u8>,
        InstructionResult,
    ) {
        let Self {
            call_stack,
            contract,
            registers,
            memory,
            result,
        } = self;

        (call_stack, contract, registers, memory, result)
    }
}

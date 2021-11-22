use crate::call::CallFrame;
use crate::consts::*;
use crate::interpreter::Interpreter;

use fuel_asm::OpcodeRepr;
use fuel_tx::ValidationError;
use fuel_types::{ContractId, RegisterId, Word};

use std::convert::Infallible;
use std::error::Error as StdError;
use std::{fmt, io};

#[derive(Debug)]
pub struct Backtrace {
    call_stack: Vec<CallFrame>,
    contract: ContractId,
    registers: [Word; VM_REGISTER_COUNT],
    memory: Vec<u8>,
    error: InterpreterError,
}

impl Backtrace {
    pub fn from_vm_error<S>(vm: &Interpreter<S>, error: InterpreterError) -> Self {
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
            error,
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

    pub const fn error(&self) -> &InterpreterError {
        &self.error
    }

    pub fn into_inner(
        self,
    ) -> (
        Vec<CallFrame>,
        ContractId,
        [Word; VM_REGISTER_COUNT],
        Vec<u8>,
        InterpreterError,
    ) {
        let Self {
            call_stack,
            contract,
            registers,
            memory,
            error,
        } = self;

        (call_stack, contract, registers, memory, error)
    }
}

impl fmt::Display for Backtrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}

impl StdError for Backtrace {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.error.source()
    }
}

#[derive(Debug)]
pub enum InterpreterError {
    OpcodeInvalid(OpcodeRepr),
    OpcodeUnimplemented(OpcodeRepr),
    ValidationError(ValidationError),
    RegisterNotWritable(RegisterId),
    Io(io::Error),
    TransactionCreateStaticContractNotFound,
    TransactionCreateIdNotInTx,
    ArithmeticOverflow,
    StackOverflow,
    PredicateOverflow,
    ProgramOverflow,
    PredicateFailure,
    ContractNotFound,
    MemoryOverflow,
    MemoryOwnership,
    ExpectedEmptyStack,
    ContractNotInTxInputs,
    NotEnoughBalance,
    ExpectedInternalContext,
    ExternalColorNotFound,
    OutOfGas,
    InputNotFound,
    OutputNotFound,
    WitnessNotFound,
    TxMaturity,
    MetadataIdentifierUndefined,

    #[cfg(feature = "debug")]
    DebugStateNotInitialized,
}

impl InterpreterError {
    pub fn backtrace<S>(self, vm: &Interpreter<S>) -> Backtrace {
        Backtrace::from_vm_error(vm, self)
    }

    /// Return if the error variant should propagate as VM panic
    pub const fn is_panic(&self) -> bool {
        matches!(
            self,
            Self::OpcodeInvalid(_)
                | Self::OpcodeUnimplemented(_)
                | Self::ValidationError(_)
                | Self::RegisterNotWritable(_)
                | Self::Io(_)
        )
    }
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidationError(e) => {
                write!(f, "Failed to validate the transaction: {}", e)
            }

            Self::Io(e) => {
                write!(f, "I/O failure: {}", e)
            }

            _ => write!(f, "Execution error: {:?}", self),
        }
    }
}

impl StdError for InterpreterError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::ValidationError(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ValidationError> for InterpreterError {
    fn from(e: ValidationError) -> Self {
        Self::ValidationError(e)
    }
}

impl From<io::Error> for InterpreterError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<Infallible> for InterpreterError {
    fn from(_i: Infallible) -> InterpreterError {
        unreachable!()
    }
}

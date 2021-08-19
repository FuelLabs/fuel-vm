use crate::consts::*;
use crate::debug::Debugger;

use fuel_asm::{RegisterId, Word};
use fuel_tx::Transaction;

mod alu;
mod blockchain;
mod contract;
mod crypto;
mod error;
mod executors;
mod flow;
mod frame;
mod gas;
mod internal;
mod log;
mod memory;
mod transaction;

#[cfg(feature = "debug")]
mod debug;

pub use contract::Contract;
pub use error::ExecuteError;
pub use executors::{ProgramState, StateTransition, StateTransitionRef};
pub use frame::{Call, CallFrame};
pub use gas::GasUnit;
pub use internal::Context;
pub use log::LogEvent;
pub use memory::MemoryRange;

#[derive(Debug, Clone)]
pub struct Interpreter<S> {
    registers: [Word; VM_REGISTER_COUNT],
    memory: Vec<u8>,
    frames: Vec<CallFrame>,
    log: Vec<LogEvent>,
    tx: Transaction,
    storage: S,
    debugger: Debugger,
    context: Context,
    block_height: u32,
}

impl<S> Interpreter<S> {
    pub fn with_storage(storage: S) -> Self {
        Self {
            registers: [0; VM_REGISTER_COUNT],
            memory: vec![0; VM_MAX_RAM as usize],
            frames: vec![],
            log: vec![],
            tx: Transaction::default(),
            storage,
            debugger: Debugger::default(),
            context: Context::default(),
            block_height: 0,
        }
    }

    pub fn memory(&self) -> &[u8] {
        self.memory.as_slice()
    }

    pub const fn registers(&self) -> &[Word] {
        &self.registers
    }

    // TODO convert to private scope after using internally
    pub const fn is_unsafe_math(&self) -> bool {
        self.registers[REG_FLAG] & 0x01 == 0x01
    }

    // TODO convert to private scope after using internally
    pub const fn is_wrapping(&self) -> bool {
        self.registers[REG_FLAG] & 0x02 == 0x02
    }

    pub fn log(&self) -> &[LogEvent] {
        self.log.as_slice()
    }
}

impl<S> From<Interpreter<S>> for Transaction {
    fn from(vm: Interpreter<S>) -> Self {
        vm.tx
    }
}

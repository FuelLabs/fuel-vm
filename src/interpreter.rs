use crate::consts::*;
use crate::debug::Debugger;

use fuel_asm::{RegisterId, Word};
use fuel_tx::consts::*;
use fuel_tx::{Bytes32, Color, Transaction};

use std::mem;

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

pub use contract::{Contract, ContractData, ContractState};
pub use error::ExecuteError;
pub use executors::{ProgramState, StateTransition, StateTransitionRef};
pub use frame::{Call, CallFrame};
pub use gas::GasUnit;
pub use internal::Context;
pub use log::LogEvent;
pub use memory::MemoryRange;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[derive(Debug, Clone)]
pub struct Interpreter<S> {
    registers: [Word; VM_REGISTER_COUNT],
    memory: Vec<u8>,
    frames: Vec<CallFrame>,
    log: Vec<LogEvent>,
    // TODO review all opcodes that mutates the tx in the stack and keep this one sync
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

    pub const fn tx_mem_address() -> usize {
        Bytes32::size_of() // Tx ID
            + WORD_SIZE // Tx size
            + MAX_INPUTS as usize * (Color::size_of() + WORD_SIZE) // Color/Balance
                                                                   // coin input
                                                                   // pairs
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

impl<S> AsMut<S> for Interpreter<S> {
    fn as_mut(&mut self) -> &mut S {
        &mut self.storage
    }
}

impl<S> From<Interpreter<S>> for Transaction {
    fn from(vm: Interpreter<S>) -> Self {
        vm.tx
    }
}

//! [`Interpreter`] implementation

use crate::call::CallFrame;
use crate::consts::*;
use crate::context::Context;
use crate::state::Debugger;

use fuel_tx::{Receipt, Transaction};
use fuel_types::Word;

mod alu;
mod blockchain;
mod constructors;
mod contract;
mod crypto;
mod executors;
mod flow;
mod frame;
mod gas;
mod initialization;
mod internal;
mod log;
mod memory;
mod metadata;
mod transaction;

#[cfg(feature = "debug")]
mod debug;

pub use memory::MemoryRange;
pub use metadata::InterpreterMetadata;

#[derive(Debug, Clone)]
pub struct Interpreter<S> {
    registers: [Word; VM_REGISTER_COUNT],
    memory: Vec<u8>,
    frames: Vec<CallFrame>,
    receipts: Vec<Receipt>,
    tx: Transaction,
    storage: S,
    debugger: Debugger,
    context: Context,
    block_height: u32,
}

impl<S> Interpreter<S> {
    pub fn memory(&self) -> &[u8] {
        self.memory.as_slice()
    }

    pub const fn registers(&self) -> &[Word] {
        &self.registers
    }

    pub fn call_stack(&self) -> &[CallFrame] {
        self.frames.as_slice()
    }

    pub const fn debugger(&self) -> &Debugger {
        &self.debugger
    }

    // TODO use this in ALU
    #[allow(dead_code)]
    pub(crate) const fn is_unsafe_math(&self) -> bool {
        self.registers[REG_FLAG] & 0x01 == 0x01
    }

    // TODO use this in ALU
    #[allow(dead_code)]
    pub(crate) const fn is_wrapping(&self) -> bool {
        self.registers[REG_FLAG] & 0x02 == 0x02
    }

    pub fn receipts(&self) -> &[Receipt] {
        self.receipts.as_slice()
    }
}

impl<S> From<Interpreter<S>> for Transaction {
    fn from(vm: Interpreter<S>) -> Self {
        vm.tx
    }
}

impl<S> AsRef<S> for Interpreter<S> {
    fn as_ref(&self) -> &S {
        &self.storage
    }
}

impl<S> AsMut<S> for Interpreter<S> {
    fn as_mut(&mut self) -> &mut S {
        &mut self.storage
    }
}

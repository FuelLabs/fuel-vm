//! [`Interpreter`] implementation

use crate::call::CallFrame;
use crate::consts::*;
use crate::context::Context;
use crate::state::Debugger;
use std::collections::HashMap;

use fuel_tx::{Receipt, Transaction};
use fuel_types::{AssetId, Word};

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
mod post_execution;
mod transaction;

#[cfg(feature = "debug")]
mod debug;

#[cfg(feature = "profile-any")]
use crate::profiler::{InstructionLocation, Profiler};

pub use memory::MemoryRange;
pub use metadata::InterpreterMetadata;

#[derive(Debug, Clone)]
/// VM interpreter.
///
/// The internal state of the VM isn't expose because the intended usage is to
/// either inspect the resulting receipts after a transaction execution, or the
/// resulting mutated transaction.
///
/// These can be obtained with the help of a [`crate::transactor::Transactor`]
/// or a client implementation.
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
    #[cfg(feature = "profile-any")]
    profiler: Profiler,
    // track the offset for each unused balance in memory
    unused_balance_index: HashMap<AssetId, usize>,
}

impl<S> Interpreter<S> {
    /// Returns the current state of the VM memory
    pub fn memory(&self) -> &[u8] {
        self.memory.as_slice()
    }

    /// Returns the current state of the registers
    pub const fn registers(&self) -> &[Word] {
        &self.registers
    }

    pub(crate) fn call_stack(&self) -> &[CallFrame] {
        self.frames.as_slice()
    }

    /// Debug handler
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

    /// The current transaction
    pub fn transaction(&self) -> &Transaction {
        &self.tx
    }

    /// Receipts generated by a transaction execution.
    pub fn receipts(&self) -> &[Receipt] {
        self.receipts.as_slice()
    }

    #[cfg(feature = "profile-any")]
    fn current_location(&self) -> InstructionLocation {
        use crate::consts::*;
        InstructionLocation::new(
            self.frames.last().map(|frame| *frame.to()),
            self.registers[REG_PC] - self.registers[REG_IS],
        )
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

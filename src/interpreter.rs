//! [`Interpreter`] implementation

use crate::call::CallFrame;
use crate::consts::*;
use crate::context::Context;
use crate::state::Debugger;

use fuel_tx::{CheckedTransaction, ConsensusParameters, Receipt, Transaction};
use fuel_types::Word;

mod alu;
mod balances;
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

#[cfg(feature = "debug")]
mod debug;

#[cfg(feature = "profile-any")]
use crate::profiler::Profiler;

#[cfg(feature = "profile-gas")]
use crate::profiler::InstructionLocation;

pub use balances::RuntimeBalances;
pub use memory::MemoryRange;

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
    tx: CheckedTransaction,
    storage: S,
    debugger: Debugger,
    context: Context,
    balances: RuntimeBalances,
    #[cfg(feature = "profile-any")]
    profiler: Profiler,
    params: ConsensusParameters,
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

    pub(crate) const fn is_unsafe_math(&self) -> bool {
        self.registers[REG_FLAG] & 0x01 == 0x01
    }

    pub(crate) const fn is_wrapping(&self) -> bool {
        self.registers[REG_FLAG] & 0x02 == 0x02
    }

    /// The current transaction
    pub fn transaction(&self) -> &Transaction {
        self.tx.as_ref()
    }

    /// The current transaction with checked metadata
    pub fn checked_transaction(&self) -> &CheckedTransaction {
        &self.tx
    }

    /// Consensus parameters
    pub const fn params(&self) -> &ConsensusParameters {
        &self.params
    }

    /// Receipts generated by a transaction execution.
    pub fn receipts(&self) -> &[Receipt] {
        self.receipts.as_slice()
    }

    #[cfg(feature = "profile-gas")]
    fn current_location(&self) -> InstructionLocation {
        use crate::consts::*;
        InstructionLocation::new(
            self.frames.last().map(|frame| *frame.to()),
            self.registers[REG_PC] - self.registers[REG_IS],
        )
    }

    /// Reference to the underlying profiler
    #[cfg(feature = "profile-any")]
    pub const fn profiler(&self) -> &Profiler {
        &self.profiler
    }
}

impl<S> From<Interpreter<S>> for CheckedTransaction {
    fn from(vm: Interpreter<S>) -> Self {
        vm.tx
    }
}

impl<S> From<Interpreter<S>> for Transaction {
    fn from(vm: Interpreter<S>) -> Self {
        vm.tx.transaction().clone()
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

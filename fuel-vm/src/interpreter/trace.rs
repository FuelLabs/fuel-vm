//! Execution traces

use super::{
    Interpreter,
    Memory,
    NoTrace,
};

/// Hooks called at specific points during the execution.
/// Can be used to inspect the state of the VM.
/// Mutable access to the vm is provided so that the state of the tracer itself can be
/// modified.
pub trait ExecutionTraceHooks: Clone
where
    Self: Sized,
{
    /// Runs after each instruction, unless that instruction enters a debugger pause
    /// state, or causes a non-well-formed panic.
    fn before_instruction<M, S, Tx, Ecal, Trace>(
        vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
    ) where
        M: Memory;
    /// Runs before each instruction
    fn after_instruction<M, S, Tx, Ecal, Trace>(
        vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
    ) where
        M: Memory;
}

impl ExecutionTraceHooks for NoTrace {
    fn before_instruction<M, S, Tx, Ecal, Trace>(
        _vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
    ) {
    }

    fn after_instruction<M, S, Tx, Ecal, Trace>(
        _vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
    ) {
    }
}

impl<M, S, Tx, Ecal, Trace> Interpreter<M, S, Tx, Ecal, Trace> {
    /// Replace trace hook type and state with a new one, discarding the old one.
    pub fn with_trace_hooks<NewTrace>(
        self,
        trace: NewTrace,
    ) -> Interpreter<M, S, Tx, Ecal, NewTrace> {
        Interpreter {
            registers: self.registers,
            memory: self.memory,
            frames: self.frames,
            receipts: self.receipts,
            tx: self.tx,
            initial_balances: self.initial_balances,
            input_contracts: self.input_contracts,
            input_contracts_index_to_output_index: self
                .input_contracts_index_to_output_index,
            storage: self.storage,
            debugger: self.debugger,
            context: self.context,
            balances: self.balances,
            profiler: self.profiler,
            interpreter_params: self.interpreter_params,
            panic_context: self.panic_context,
            ecal_state: self.ecal_state,
            trace,
        }
    }

    /// Read access to the trace state
    pub fn trace(&self) -> &Trace {
        &self.trace
    }

    /// Write access to the trace state
    pub fn trace_mut(&mut self) -> &mut Trace {
        &mut self.trace
    }
}

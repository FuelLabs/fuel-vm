//! Execution traces

use super::{
    Interpreter,
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
    /// Runs before each instruction
    fn after_instruction<M, S, Tx, Ecal, Trace>(
        vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
    );
    /// Runs after each instruction, unless that instruction enters a debugger pause
    /// state, or causes a non-well-formed panic.
    fn before_instruction<M, S, Tx, Ecal, Trace>(
        vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
    );
}

impl ExecutionTraceHooks for NoTrace {
    fn after_instruction<M, S, Tx, Ecal, Trace>(
        _vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
    ) {
    }

    fn before_instruction<M, S, Tx, Ecal, Trace>(
        _vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
    ) {
    }
}

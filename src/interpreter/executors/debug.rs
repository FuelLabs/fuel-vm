use crate::error::InterpreterError;
use crate::interpreter::{ExecutableTransaction, Interpreter};
use crate::state::ProgramState;
use crate::storage::InterpreterStorage;

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    /// Continue the execution from a previously interrupted program flow.
    pub fn resume(&mut self) -> Result<ProgramState, InterpreterError> {
        let state = self
            .debugger_last_state()
            .ok_or(InterpreterError::DebugStateNotInitialized)?;

        let state = match state {
            ProgramState::Return(w) => Ok(ProgramState::Return(w)),

            ProgramState::ReturnData(d) => Ok(ProgramState::ReturnData(d)),

            ProgramState::Revert(w) => Ok(ProgramState::Revert(w)),

            ProgramState::RunProgram(_) => self.run_program(),

            ProgramState::VerifyPredicate(_) => unimplemented!(),
        }?;

        if state.is_debug() {
            self.debugger_set_last_state(state.clone());
        }

        Ok(state)
    }
}

use super::ProgramState;
use crate::data::InterpreterStorage;
use crate::interpreter::{ExecuteError, Interpreter};

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub fn resume(&mut self) -> Result<ProgramState, ExecuteError> {
        let state = self
            .debugger_last_state()
            .ok_or(ExecuteError::DebugStateNotInitialized)?;

        let state = match state {
            ProgramState::Return(w) => Ok(ProgramState::Return(w)),

            ProgramState::ReturnData(d) => Ok(ProgramState::ReturnData(d)),

            ProgramState::RunProgram(_) => self.run_program(),

            ProgramState::VerifyPredicate(_) => unimplemented!(),
        }?;

        if state.is_debug() {
            self.debugger_set_last_state(state.clone());
        }

        Ok(state)
    }
}

use crate::{
    error::InterpreterError,
    interpreter::{
        EcalHandler,
        ExecutableTransaction,
        Interpreter,
        Memory,
    },
    state::ProgramState,
    storage::InterpreterStorage,
    verification::Verifier,
};
use fuel_tx::field::Inputs;

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Tx: Inputs,
    Ecal: EcalHandler,
    V: Verifier,
{
    /// Continue the execution from a previously interrupted program flow.
    pub fn resume(&mut self) -> Result<ProgramState, InterpreterError<S::DataError>> {
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
            self.debugger_set_last_state(state);
        }

        Ok(state)
    }
}

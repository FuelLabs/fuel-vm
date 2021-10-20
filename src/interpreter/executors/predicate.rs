use crate::consts::*;
use crate::error::InterpreterError;
use crate::interpreter::{Interpreter, MemoryRange};
use crate::state::{ExecuteState, ProgramState};
use crate::storage::InterpreterStorage;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn verify_predicate(&mut self, predicate: &MemoryRange) -> Result<ProgramState, InterpreterError> {
        // TODO initialize VM with tx prepared for sign
        // TODO execute should not overflow predicate boundaries. Need to check
        // internally if a Jump instruction decrements $pc, or if $pc overflows
        // `end`
        let (start, _end) = predicate.boundaries(self);

        self.registers[REG_PC] = start;
        self.registers[REG_IS] = start;

        // TODO optimize
        loop {
            match self.execute()? {
                ExecuteState::Return(r) => {
                    if r == 1 {
                        return Ok(ProgramState::Return(r));
                    } else {
                        return Err(InterpreterError::PredicateFailure);
                    }
                }

                // A predicate is not expected to return data
                ExecuteState::ReturnData(_) => return Err(InterpreterError::PredicateFailure),

                ExecuteState::Revert(r) => return Ok(ProgramState::Revert(r)),

                ExecuteState::Proceed => (),

                #[cfg(feature = "debug")]
                ExecuteState::DebugEvent(d) => {
                    return Ok(ProgramState::VerifyPredicate(d));
                }
            }
        }
    }
}

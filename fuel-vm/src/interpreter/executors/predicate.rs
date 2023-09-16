use crate::{
    error::InterpreterError,
    prelude::{
        CheckError,
        ExecutableTransaction,
        Interpreter,
    },
    state::{
        ExecuteState,
        ProgramState,
    },
    storage::PredicateStorage,
};

use fuel_asm::{
    PanicReason,
    RegId,
};

impl<Tx> Interpreter<PredicateStorage, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn verify_predicate(&mut self) -> Result<ProgramState, InterpreterError> {
        let range = self
            .context
            .predicate()
            .ok_or(InterpreterError::CheckError(
                CheckError::PredicateVerificationFailed,
            ))?
            .program()
            .words();

        self.registers[RegId::PC] = range.start;
        self.registers[RegId::IS] = range.start;

        loop {
            if range.end <= self.registers[RegId::PC] {
                return Err(InterpreterError::Panic(PanicReason::MemoryOverflow))
            }

            match self.execute()? {
                ExecuteState::Return(r) => {
                    if r == 1 {
                        return Ok(ProgramState::Return(r))
                    } else {
                        return Err(InterpreterError::Panic(
                            PanicReason::ContractInstructionNotAllowed,
                        ))
                    }
                }

                // A predicate is not expected to return data
                ExecuteState::ReturnData(_) => {
                    return Err(InterpreterError::Panic(
                        PanicReason::ContractInstructionNotAllowed,
                    ))
                }

                ExecuteState::Revert(r) => return Ok(ProgramState::Revert(r)),

                ExecuteState::Proceed => (),

                ExecuteState::DebugEvent(d) => {
                    return Ok(ProgramState::VerifyPredicate(d))
                }
            }
        }
    }
}

use crate::{
    error::{
        InterpreterError,
        PredicateVerificationFailed,
    },
    interpreter::EcalHandler,
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

impl<Tx, Ecal> Interpreter<PredicateStorage, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    pub(crate) fn verify_predicate(
        &mut self,
    ) -> Result<ProgramState, PredicateVerificationFailed> {
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
                return Err(PanicReason::MemoryOverflow.into())
            }

            match self.execute()? {
                ExecuteState::Return(r) => {
                    if r == 1 {
                        return Ok(ProgramState::Return(r))
                    } else {
                        return Err(PanicReason::PredicateReturnedNonOne.into())
                    }
                }

                // A predicate is not expected to return data
                ExecuteState::ReturnData(_) => {
                    return Err(PanicReason::ContractInstructionNotAllowed.into())
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

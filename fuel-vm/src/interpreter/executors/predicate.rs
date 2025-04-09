use crate::{
    error::InterpreterError,
    interpreter::{
        EcalHandler,
        Memory,
    },
    prelude::{
        ExecutableTransaction,
        Interpreter,
    },
    state::{
        ExecuteState,
        ProgramState,
    },
    storage::predicate::{
        PredicateStorage,
        PredicateStorageError,
    },
};

use crate::storage::predicate::PredicateStorageRequirements;
use fuel_asm::PanicReason;

impl<M, Tx, Ecal, S> Interpreter<M, PredicateStorage<S>, Tx, Ecal>
where
    M: Memory,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
    S: PredicateStorageRequirements,
{
    /// Verify a predicate that has been initialized already
    pub(crate) fn verify_predicate(
        &mut self,
    ) -> Result<ProgramState, InterpreterError<PredicateStorageError>> {
        loop {
            match self.execute::<true>()? {
                ExecuteState::Return(r) => {
                    if r == 1 {
                        return Ok(ProgramState::Return(r))
                    } else {
                        return Err(InterpreterError::Panic(
                            PanicReason::PredicateReturnedNonOne,
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

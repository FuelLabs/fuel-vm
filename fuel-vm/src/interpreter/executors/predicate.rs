use crate::{
    error::PredicateVerificationFailed,
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
    storage::predicate::PredicateStorage,
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
    ) -> Result<ProgramState, PredicateVerificationFailed> {
        loop {
            match self.execute::<false>()? {
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

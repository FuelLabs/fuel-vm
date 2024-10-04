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
    storage::PredicateStorage,
};
use core::fmt::Debug;

use crate::storage::BlobData;
use fuel_asm::PanicReason;
use fuel_storage::StorageRead;

impl<M, Tx, Ecal, S> Interpreter<M, PredicateStorage<S>, Tx, Ecal>
where
    M: Memory,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
    S: StorageRead<BlobData>,
    S::Error: Debug,
{
    /// Verify a predicate that has been initialized already
    pub(crate) fn verify_predicate(
        &mut self,
    ) -> Result<ProgramState, PredicateVerificationFailed> {
        loop {
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

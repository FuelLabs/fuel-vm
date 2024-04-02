use crate::{
    error::PredicateVerificationFailed,
    interpreter::EcalHandler,
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

use fuel_asm::{
    PanicReason,
    RegId,
    Word,
};

impl<Tx, Ecal> Interpreter<PredicateStorage, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    /// Verify a predicate that has been initialized already
    pub(crate) fn verify_predicate(
        &mut self,
    ) -> Result<ProgramState, PredicateVerificationFailed> {
        let range = self
            .context
            .predicate()
            .expect("The predicate is not initialized")
            .program()
            .words();

        self.registers[RegId::PC] = range.start as Word;
        self.registers[RegId::IS] = range.start as Word;

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

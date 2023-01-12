use crate::consts::*;
use crate::error::InterpreterError;
use crate::prelude::{ExecutableTransaction, Interpreter};
use crate::state::{ExecuteState, ProgramState};
use crate::storage::PredicateStorage;

use fuel_asm::PanicReason;

impl<Tx> Interpreter<PredicateStorage, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn verify_predicate(&mut self) -> Result<ProgramState, InterpreterError> {
        let (start, end) = self
            .context
            .predicate()
            .map(|p| p.program().boundaries(&self.ownership_registers()))
            .ok_or(InterpreterError::PredicateFailure)?;

        self.registers[REG_PC] = start;
        self.registers[REG_IS] = start;

        loop {
            if end <= self.registers[REG_PC] {
                return Err(InterpreterError::Panic(PanicReason::MemoryOverflow));
            }

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

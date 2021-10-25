use crate::consts::*;
use crate::error::InterpreterError;
use crate::interpreter::{Interpreter, MemoryRange};
use crate::state::{ExecuteState, ProgramState};
use crate::storage::InterpreterStorage;

use fuel_asm::Opcode;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub(crate) fn verify_predicate(&mut self, predicate: &MemoryRange) -> Result<ProgramState, InterpreterError> {
        // TODO initialize VM with tx prepared for sign
        let (start, end) = predicate.boundaries(self);

        self.registers[REG_PC] = start;
        self.registers[REG_IS] = start;

        // TODO optimize
        loop {
            let pc = self.registers[REG_PC];
            let op = self.memory[pc as usize..]
                .chunks_exact(Opcode::BYTES_SIZE)
                .next()
                .map(Opcode::from_bytes_unchecked)
                .ok_or(InterpreterError::PredicateOverflow)?;

            match self.execute(op)? {
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

            if self.registers[REG_PC] < pc || self.registers[REG_PC] >= end {
                return Err(InterpreterError::PredicateOverflow);
            }
        }
    }
}

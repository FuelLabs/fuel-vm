use super::Interpreter;
use crate::consts::*;
use crate::error::InterpreterError;

use fuel_types::{Immediate18, RegisterId, Word};

use std::convert::TryFrom;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpreterMetadata {
    IsCallerExternal = 0x000001,
    GetCaller = 0x000002,
}

impl TryFrom<Immediate18> for InterpreterMetadata {
    type Error = InterpreterError;

    fn try_from(imm: Immediate18) -> Result<Self, Self::Error> {
        match imm {
            0x000001 => Ok(Self::IsCallerExternal),
            0x000002 => Ok(Self::GetCaller),
            _ => Err(InterpreterError::MetadataIdentifierUndefined),
        }
    }
}

impl From<InterpreterMetadata> for Immediate18 {
    fn from(m: InterpreterMetadata) -> Immediate18 {
        match m {
            InterpreterMetadata::IsCallerExternal => 0x000001,
            InterpreterMetadata::GetCaller => 0x000002,
        }
    }
}

impl<S> Interpreter<S> {
    pub(crate) fn metadata(&mut self, ra: RegisterId, imm: Immediate18) -> Result<(), InterpreterError> {
        // Both metadata implementations should panic if external context
        if self.is_external_context() {
            return Err(InterpreterError::ExpectedInternalContext);
        }

        let metadata = InterpreterMetadata::try_from(imm)?;
        let parent = self.frames.last().map(|f| f.registers()[REG_FP]).unwrap_or(0);

        match metadata {
            InterpreterMetadata::IsCallerExternal => {
                self.registers[ra] = (parent != 0) as Word;
            }

            InterpreterMetadata::GetCaller => {
                if parent == 0 {
                    return Err(InterpreterError::ExpectedInternalContext);
                }

                self.registers[ra] = parent;
            }
        }

        self.inc_pc()?;

        Ok(())
    }
}

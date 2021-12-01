use super::Interpreter;
use crate::consts::*;

use fuel_asm::PanicReason;
use fuel_types::{Immediate18, RegisterId, Word};

const IS_CALLER_EXTERNAL: Immediate18 = 0x000001;
const GET_CALLER: Immediate18 = 0x000002;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpreterMetadata {
    IsCallerExternal = IS_CALLER_EXTERNAL as isize,
    GetCaller = GET_CALLER as isize,
}

impl TryFrom<Immediate18> for InterpreterMetadata {
    type Error = PanicReason;

    fn try_from(imm: Immediate18) -> Result<Self, Self::Error> {
        match imm {
            IS_CALLER_EXTERNAL => Ok(Self::IsCallerExternal),
            GET_CALLER => Ok(Self::GetCaller),
            _ => Err(PanicReason::InvalidMetadataIdentifier),
        }
    }
}

impl From<InterpreterMetadata> for Immediate18 {
    fn from(m: InterpreterMetadata) -> Immediate18 {
        match m {
            InterpreterMetadata::IsCallerExternal => IS_CALLER_EXTERNAL,
            InterpreterMetadata::GetCaller => GET_CALLER,
        }
    }
}

impl<S> Interpreter<S> {
    pub(crate) fn metadata(&mut self, ra: RegisterId, imm: Immediate18) -> Result<(), PanicReason> {
        Self::is_register_writable(ra)?;

        // Both metadata implementations should panic if external context
        if self.is_external_context() {
            return Err(PanicReason::ExpectedInternalContext);
        }

        let parent = self
            .frames
            .last()
            .map(|f| f.registers()[REG_FP])
            .expect("External context will always have a frame");

        match imm {
            IS_CALLER_EXTERNAL => {
                self.registers[ra] = (parent != 0) as Word;
            }

            GET_CALLER => {
                if parent == 0 {
                    return Err(PanicReason::ExpectedInternalContext);
                }

                self.registers[ra] = parent;
            }

            _ => return Err(PanicReason::InvalidMetadataIdentifier),
        }

        self.inc_pc()
    }
}

use super::Interpreter;
use crate::consts::*;
use crate::error::RuntimeError;

use fuel_asm::{GMArgs, PanicReason};
use fuel_types::{Immediate18, RegisterId, Word};

impl<S> Interpreter<S> {
    pub(crate) fn metadata(&mut self, ra: RegisterId, imm: Immediate18) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;

        // Both metadata implementations should panic if external context
        if self.is_external_context() {
            return Err(PanicReason::ExpectedInternalContext.into());
        }

        let parent = self
            .frames
            .last()
            .map(|f| f.registers()[REG_FP])
            .expect("External context will always have a frame");

        match GMArgs::try_from(imm)? {
            GMArgs::IsCallerExternal => {
                self.registers[ra] = (parent == 0) as Word;
            }

            GMArgs::GetCaller if parent == 0 => {
                return Err(PanicReason::ExpectedInternalContext.into());
            }

            GMArgs::GetCaller => {
                self.registers[ra] = parent;
            }

            GMArgs::GetVerifyingPredicate => todo!(),
        }

        self.inc_pc()
    }
}

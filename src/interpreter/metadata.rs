use super::Interpreter;
use crate::consts::*;
use crate::error::RuntimeError;

use fuel_asm::{GMArgs, PanicReason};
use fuel_types::{Immediate18, RegisterId, Word};

impl<S> Interpreter<S> {
    pub(crate) fn metadata(&mut self, ra: RegisterId, imm: Immediate18) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;

        let external = self.is_external_context();
        let args = GMArgs::try_from(imm)?;

        if external {
            match args {
                GMArgs::GetVerifyingPredicate => {
                    self.registers[ra] = self
                        .context
                        .predicate()
                        .map(|p| p.idx() as Word)
                        .ok_or(PanicReason::TransactionValidity)?;
                }

                _ => return Err(PanicReason::ExpectedInternalContext.into()),
            }
        } else {
            let parent = self
                .frames
                .last()
                .map(|f| f.registers()[REG_FP])
                .expect("External context will always have a frame");

            match args {
                GMArgs::IsCallerExternal => {
                    self.registers[ra] = (parent == 0) as Word;
                }

                GMArgs::GetCaller if parent != 0 => {
                    self.registers[ra] = parent;
                }

                _ => return Err(PanicReason::ExpectedInternalContext.into()),
            }
        }

        self.inc_pc()
    }
}

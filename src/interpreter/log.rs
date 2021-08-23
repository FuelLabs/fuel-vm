use super::Interpreter;
use crate::consts::*;

use fuel_asm::{RegisterId, Word};
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum LogEvent {
    Register {
        pc: Word,
        register: RegisterId,
        value: Word,
    },

    Return {
        register: RegisterId,
        value: Word,
    },
}

impl LogEvent {
    pub const fn value(&self) -> Word {
        match self {
            Self::Register { value, .. } => *value,
            Self::Return { value, .. } => *value,
        }
    }
}

impl<S> Interpreter<S> {
    pub(crate) fn log_append(&mut self, reg: &[RegisterId]) -> bool {
        let pc = self.registers[REG_PC];
        let registers = &self.registers;
        let log = &mut self.log;

        let entries = reg.iter().filter(|r| r > &&0).filter_map(|r| {
            registers.get(*r).map(|v| {
                let log = LogEvent::Register {
                    pc,
                    register: *r,
                    value: *v,
                };

                debug!("Appending log {:?}", log);
                log
            })
        });

        log.extend(entries);

        true
    }

    pub(crate) fn log_return(&mut self, register: RegisterId) -> bool {
        match self.registers.get(register as usize).copied() {
            Some(value) => {
                self.log.push(LogEvent::Return { register, value });
                true
            }

            _ => false,
        }
    }
}

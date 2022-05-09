use super::Interpreter;
use crate::consts::*;
use crate::error::RuntimeError;

use fuel_asm::PanicReason;
use fuel_types::{RegisterId, Word};

impl<S> Interpreter<S> {
    pub(crate) fn alu_overflow<F, B, C>(&mut self, ra: RegisterId, f: F, b: B, c: C) -> Result<(), RuntimeError>
    where
        F: FnOnce(B, C) -> (Word, bool),
    {
        Self::is_register_writable(ra)?;

        let (result, overflow) = f(b, c);

        if overflow && !self.is_wrapping() {
            return Err(PanicReason::ArithmeticOverflow.into());
        }

        self.registers[REG_OF] = overflow as Word;
        self.registers[REG_ERR] = 0;

        self.registers[ra] = result;

        self.inc_pc()
    }

    pub(crate) fn alu_error<F, B, C>(&mut self, ra: RegisterId, f: F, b: B, c: C, err: bool) -> Result<(), RuntimeError>
    where
        F: FnOnce(B, C) -> Word,
    {
        Self::is_register_writable(ra)?;

        if err && !self.is_unsafe_math() {
            return Err(PanicReason::ErrorFlag.into());
        }

        self.registers[REG_OF] = 0;
        self.registers[REG_ERR] = err as Word;

        self.registers[ra] = if err { 0 } else { f(b, c) };

        self.inc_pc()
    }

    pub(crate) fn alu_set(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;

        self.registers[REG_OF] = 0;
        self.registers[REG_ERR] = 0;

        self.registers[ra] = b;

        self.inc_pc()
    }

    pub(crate) fn alu_clear(&mut self) -> Result<(), RuntimeError> {
        self.registers[REG_OF] = 0;
        self.registers[REG_ERR] = 0;

        self.inc_pc()
    }
}

use super::Interpreter;
use crate::consts::*;
use crate::error::RuntimeError;

use fuel_asm::PanicReason;
use fuel_types::{RegisterId, Word};

impl<S> Interpreter<S> {
    /// Stores the overflowed wrapped value into REG_OF
    pub(crate) fn alu_capture_overflow<F, B, C>(&mut self, ra: RegisterId, f: F, b: B, c: C) -> Result<(), RuntimeError>
    where
        F: FnOnce(B, C) -> (u128, bool),
    {
        Self::is_register_writable(ra)?;

        let (result, _overflow) = f(b, c);

        if result > Word::MAX as u128 && !self.is_wrapping() {
            return Err(PanicReason::ArithmeticOverflow.into());
        }

        // set the OF register to high bits of the u128 result
        self.registers[REG_OF] = (result >> 64) as u64;
        self.registers[REG_ERR] = 0;

        // set the return value to the low bits of the u128 result
        self.registers[ra] = (result & Word::MAX as u128) as u64;

        self.inc_pc()
    }

    /// Set REG_OF to true and zero the result register if overflow occurred.
    pub(crate) fn alu_boolean_overflow<F, B, C>(&mut self, ra: RegisterId, f: F, b: B, c: C) -> Result<(), RuntimeError>
    where
        F: FnOnce(B, C) -> (Word, bool),
    {
        Self::is_register_writable(ra)?;

        let (result, overflow) = f(b, c);

        if overflow && !self.is_wrapping() {
            return Err(PanicReason::ArithmeticOverflow.into());
        }

        // set the OF register to 1 if an overflow occurred
        self.registers[REG_OF] = overflow as Word;
        self.registers[REG_ERR] = 0;

        self.registers[ra] = if overflow { 0 } else { result };

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

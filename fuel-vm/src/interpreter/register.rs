use fuel_asm::{
    Instruction,
    RegId,
    Word,
};
use fuel_tx::PanicReason;

use crate::error::SimpleResult;

use super::Interpreter;

/// Check that the register is a general-purpose user-writable register.
pub(crate) fn verify_register_user_writable(r: RegId) -> Result<(), PanicReason> {
    if r >= RegId::WRITABLE {
        Ok(())
    } else {
        Err(PanicReason::ReservedRegisterNotWritable)
    }
}

impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal> {
    /// Returns the current state of the registers
    pub const fn registers(&self) -> &[Word] {
        &self.registers
    }

    /// Returns mutable access to the registers
    pub fn registers_mut(&mut self) -> &mut [Word] {
        &mut self.registers
    }

    /// Writes a value to an user-writable register, causing a vm panic otherwise.
    pub fn write_user_register(&mut self, reg: RegId) -> SimpleResult<&mut Word> {
        verify_register_user_writable(reg)?;
        Ok(&mut self.registers_mut()[reg.to_u8() as usize])
    }

    /// Increase program counter, causing a vm panic if the new value is out of bounds.
    pub fn inc_pc(&mut self) -> SimpleResult<()> {
        let pc = &mut self.registers[RegId::PC];
        *pc = pc
            .checked_add(Instruction::SIZE as Word)
            .ok_or(PanicReason::MemoryOverflow)?;
        Ok(())
    }
}

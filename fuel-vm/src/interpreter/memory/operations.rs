use super::super::internal::inc_pc;
use super::super::{ExecutableTransaction, Interpreter};
use super::{AllocatedPages, MemoryRange};
use crate::constraints::reg_key::*;
use crate::consts::*;
use crate::error::RuntimeError;

use fuel_asm::PanicReason;
use fuel_types::{RegisterId, Word};

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    /// Modify stack pointer using an overflowing-style arithmetic operation.
    pub(crate) fn stack_pointer_overflow<F>(&mut self, f: F, v: Word) -> Result<(), RuntimeError>
    where
        F: FnOnce(Word, Word) -> (Word, bool),
    {
        let (
            SystemRegisters {
                mut sp, ssp, hp, pc, ..
            },
            _,
        ) = split_registers(&mut self.registers);
        let (result, overflow) = f(*sp, v);

        if overflow || result >= *hp || result < *ssp {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            *sp = result;
            let _pages = self
                .memory
                .update_allocations(*sp, *hp)
                .map_err(|_| PanicReason::OutOfMemory)?;
            // TODO: gas price for the allocated pages

            inc_pc(pc)
        }
    }

    pub(crate) fn load_byte(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        *result = self.memory.at(b.saturating_add(c))? as Word;
        inc_pc(pc)
    }

    pub(crate) fn load_word(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        // C is expressed in words; mul by 8. This cannot overflow since it's a 12 bit immediate value.
        let addr = b.checked_add(c * 8).ok_or(PanicReason::MemoryOverflow)?;
        *result = Word::from_be_bytes(self.memory.read_bytes(addr)?);
        inc_pc(pc)
    }

    pub(crate) fn store_byte(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        let pc = self.registers.pc_mut();
        let (ac, overflow) = a.overflowing_add(c);
        if overflow {
            return Err(PanicReason::MemoryOverflow.into());
        }

        self.memory.set_at(owner, ac, b as u8)?;

        inc_pc(pc)
    }

    pub(crate) fn store_word(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        let pc = self.registers.pc_mut();
        // C is expressed in words; mul by 8. This cannot overflow since it's a 12 bit immediate value.
        let addr = a.checked_add(c * 8).ok_or(PanicReason::MemoryOverflow)?;
        self.memory.write_bytes(owner, addr, &b.to_be_bytes())?;
        inc_pc(pc)
    }

    pub(crate) fn malloc(&mut self, a: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { mut hp, sp, .. }, _) = split_registers(&mut self.registers);
        let (result, overflow) = hp.overflowing_sub(a);
        if overflow || result < *sp {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            *hp = result;
            let AllocatedPages(pages_allocated) = self
                .memory
                .update_allocations(*sp, *hp)
                .map_err(|_| RuntimeError::Recoverable(PanicReason::OutOfMemory))?;

            // TODO: gas price for the page
            self.gas_charge((pages_allocated as u64) * 10)?;

            let (SystemRegisters { pc, .. }, _) = split_registers(&mut self.registers);
            inc_pc(pc)
        }
    }

    pub(crate) fn memclear(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        let range = MemoryRange::try_new(a, b)?;
        self.memory.try_clear(owner, range)?;
        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn memcopy(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        self.memory.try_copy_within(owner, a, b, c)?;
        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn memeq(&mut self, ra: RegisterId, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        if d > MEM_MAX_ACCESS_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let range0 = MemoryRange::try_new(b, d)?;
        let range1 = MemoryRange::try_new(c, d)?;

        let eq = self
            .memory
            .read_range(range0)?
            .zip(self.memory.read_range(range1)?)
            .all(|(a, b)| a == b);
        *result = eq as Word;

        inc_pc(pc)
    }
}

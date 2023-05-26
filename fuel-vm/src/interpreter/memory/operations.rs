use super::super::internal::inc_pc;
use super::super::{ExecutableTransaction, Interpreter};
use super::{AllocatedPages, MemoryRange, OwnershipRegisters, VmMemory};
use crate::constraints::reg_key::*;
use crate::consts::*;
use crate::error::RuntimeError;

use fuel_asm::PanicReason;
use fuel_types::{RegisterId, Word};

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn stack_pointer_overflow<F>(&mut self, f: F, v: Word) -> Result<(), RuntimeError>
    where
        F: FnOnce(Word, Word) -> (Word, bool),
    {
        let (SystemRegisters { sp, hp, pc, .. }, _) = split_registers(&mut self.registers);
        stack_pointer_overflow(sp, hp.as_ref(), pc, f, v)
    }

    pub(crate) fn load_byte(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        load_byte(&self.memory, pc, result, b, c)
    }

    pub(crate) fn load_word(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        load_word(&self.memory, pc, result, b, c)
    }

    pub(crate) fn store_byte(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        store_byte(&mut self.memory, owner, self.registers.pc_mut(), a, b, c)
    }

    pub(crate) fn store_word(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        store_word(&mut self.memory, owner, self.registers.pc_mut(), a, b, c)
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
        self.memory.try_copy_within(owner, a as usize, b as usize, c as usize)?;
        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn memeq(&mut self, ra: RegisterId, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        memeq(&mut self.memory, result, pc, b, c, d)
    }
}

pub(crate) fn stack_pointer_overflow<F>(
    mut sp: RegMut<SP>,
    hp: Reg<HP>,
    pc: RegMut<PC>,
    f: F,
    v: Word,
) -> Result<(), RuntimeError>
where
    F: FnOnce(Word, Word) -> (Word, bool),
{
    let (result, overflow) = f(*sp, v);

    if overflow || result >= *hp {
        Err(PanicReason::MemoryOverflow.into())
    } else {
        *sp = result;

        inc_pc(pc)
    }
}

pub(crate) fn load_byte(
    memory: &VmMemory,
    pc: RegMut<PC>,
    result: &mut Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
    let bc = b.saturating_add(c) as usize;

    *result = memory.at(bc)? as Word;
    inc_pc(pc)
}

pub(crate) fn load_word(
    memory: &VmMemory,
    pc: RegMut<PC>,
    result: &mut Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
    // C is expressed in words; mul by 8. This cannot overflow since it's a 12 bit immediate value.
    let addr = b.checked_add(c * 8).ok_or(PanicReason::MemoryOverflow)?;
    *result = Word::from_be_bytes(memory.read_bytes(addr as usize)?);
    inc_pc(pc)
}

pub(crate) fn store_byte(
    memory: &mut VmMemory,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
    let (ac, overflow) = a.overflowing_add(c);
    if overflow {
        return Err(PanicReason::MemoryOverflow.into());
    }

    memory.set_at(owner, ac as usize, b as u8)?;

    inc_pc(pc)
}

pub(crate) fn store_word(
    memory: &mut VmMemory,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
    // C is expressed in words; mul by 8. This cannot overflow since it's a 12 bit immediate value.
    let addr = a.checked_add(c * 8).ok_or(PanicReason::MemoryOverflow)?;
    memory.write_bytes(owner, addr as usize, &b.to_be_bytes())?;
    inc_pc(pc)
}

pub(crate) fn memeq(
    memory: &mut VmMemory,
    result: &mut Word,
    pc: RegMut<PC>,
    b: Word,
    c: Word,
    d: Word,
) -> Result<(), RuntimeError> {
    if d > MEM_MAX_ACCESS_SIZE {
        return Err(PanicReason::MemoryOverflow.into());
    }

    let range0 = MemoryRange::try_new(b, d)?;
    let range1 = MemoryRange::try_new(c, d)?;

    let eq = memory
        .read_range(range0)?
        .zip(memory.read_range(range1)?)
        .all(|(a, b)| a == b);
    *result = eq as Word;

    inc_pc(pc)
}

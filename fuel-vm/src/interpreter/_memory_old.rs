use super::internal::inc_pc;
use super::memory::OwnershipRegisters;
use super::{ExecutableTransaction, Interpreter};
use crate::constraints::reg_key::*;
use crate::error::RuntimeError;

use fuel_asm::PanicReason;
use fuel_types::{RegisterId, Word};

pub type Memory<const SIZE: usize> = Box<[u8; SIZE]>;

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
        let (SystemRegisters { hp, sp, pc, .. }, _) = split_registers(&mut self.registers);
        malloc(hp, sp.as_ref(), pc, a)
    }

    pub(crate) fn memclear(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        memclear(&mut self.memory, owner, self.registers.pc_mut(), a, b)
    }

    pub(crate) fn memcopy(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        memcopy(&mut self.memory, owner, self.registers.pc_mut(), a, b, c)
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

// pub(crate) fn load_byte(
//     memory: &VmMemory,
//     pc: RegMut<PC>,
//     result: &mut Word,
//     b: Word,
//     c: Word,
// ) -> Result<(), RuntimeError> {
//     let bc = b.saturating_add(c) as usize;

//     if bc >= VM_MAX_RAM as RegisterId {
//         Err(PanicReason::MemoryOverflow.into())
//     } else {
//         *result = memory[bc] as Word;

//         inc_pc(pc)
//     }
// }

// pub(crate) fn load_word(
//     memory: &VmMemory,
//     pc: RegMut<PC>,
//     result: &mut Word,
//     b: Word,
//     c: Word,
// ) -> Result<(), RuntimeError> {
//     // C is expressed in words; mul by 8. This cannot overflow since it's a 12 bit immediate value.
//     let addr = b.checked_add(c * 8).ok_or(PanicReason::MemoryOverflow)?;
//     *result = Word::from_be_bytes(read_bytes(memory, addr)?);
//     inc_pc(pc)
// }

// pub(crate) fn store_byte(
//     memory: &mut VmMemory,
//     owner: OwnershipRegisters,
//     pc: RegMut<PC>,
//     a: Word,
//     b: Word,
//     c: Word,
// ) -> Result<(), RuntimeError> {
//     let (ac, overflow) = a.overflowing_add(c);
//     let range = ac..(ac + 1);
//     if overflow || ac >= VM_MAX_RAM || !(owner.has_ownership_stack(&range) || owner.has_ownership_heap(&range)) {
//         Err(PanicReason::MemoryOverflow.into())
//     } else {
//         memory[ac as usize] = b as u8;

//         inc_pc(pc)
//     }
// }

// pub(crate) fn store_word(
//     memory: &mut VmMemory,
//     owner: OwnershipRegisters,
//     pc: RegMut<PC>,
//     a: Word,
//     b: Word,
//     c: Word,
// ) -> Result<(), RuntimeError> {
//     // C is expressed in words; mul by 8. This cannot overflow since it's a 12 bit immediate value.
//     let addr = a.checked_add(c * 8).ok_or(PanicReason::MemoryOverflow)?;
//     memory.write_bytes(owner, addr, b.to_be_bytes())?;
//     inc_pc(pc)
// }

// pub(crate) fn malloc(mut hp: RegMut<HP>, sp: Reg<SP>, pc: RegMut<PC>, a: Word) -> Result<(), RuntimeError> {
//     let (result, overflow) = hp.overflowing_sub(a);

//     if overflow || result < *sp {
//         Err(PanicReason::MemoryOverflow.into())
//     } else {
//         *hp = result;

//         inc_pc(pc)
//     }
// }

// pub(crate) fn memclear(
//     memory: &mut VmMemory,
//     owner: OwnershipRegisters,
//     pc: RegMut<PC>,
//     a: Word,
//     b: Word,
// ) -> Result<(), RuntimeError> {
//     let (ab, overflow) = a.overflowing_add(b);

//     let range = MemoryRange::new(a, b);
//     if overflow || ab > VM_MAX_RAM || b > MEM_MAX_ACCESS_SIZE || !owner.has_ownership_range(&range) {
//         Err(PanicReason::MemoryOverflow.into())
//     } else {
//         // trivial compiler optimization for memset
//         for i in &mut memory[a as usize..ab as usize] {
//             *i = 0
//         }

//         inc_pc(pc)
//     }
// }

// pub(crate) fn memcopy(
//     memory: &mut VmMemory,
//     owner: OwnershipRegisters,
//     pc: RegMut<PC>,
//     a: Word,
//     b: Word,
//     c: Word,
// ) -> Result<(), RuntimeError> {
//     let (ac, overflow) = a.overflowing_add(c);
//     let (bc, of) = b.overflowing_add(c);
//     let overflow = overflow || of;

//     let range = MemoryRange::new(a, c);
//     if overflow
//         || ac > VM_MAX_RAM
//         || bc > VM_MAX_RAM
//         || c > MEM_MAX_ACCESS_SIZE
//         || a <= b && b < ac
//         || b <= a && a < bc
//         || a < bc && bc <= ac
//         || b < ac && ac <= bc
//         || !owner.has_ownership_range(&range)
//     {
//         Err(PanicReason::MemoryOverflow.into())
//     } else {
//         if a <= b {
//             let (dst, src) = memory.split_at_mut(b as usize);
//             dst[a as usize..ac as usize].copy_from_slice(&src[..c as usize]);
//         } else {
//             let (src, dst) = memory.split_at_mut(a as usize);
//             dst[..c as usize].copy_from_slice(&src[b as usize..bc as usize]);
//         }

//         inc_pc(pc)
//     }
// }

// pub(crate) fn memeq(
//     memory: &mut VmMemory,
//     result: &mut Word,
//     pc: RegMut<PC>,
//     b: Word,
//     c: Word,
//     d: Word,
// ) -> Result<(), RuntimeError> {
//     let (bd, overflow) = b.overflowing_add(d);
//     let (cd, of) = c.overflowing_add(d);
//     let overflow = overflow || of;

//     if overflow || bd > VM_MAX_RAM || cd > VM_MAX_RAM || d > MEM_MAX_ACCESS_SIZE {
//         Err(PanicReason::MemoryOverflow.into())
//     } else {
//         *result = (memory[b as usize..bd as usize] == memory[c as usize..cd as usize]) as Word;

//         inc_pc(pc)
//     }
// }

// pub(crate) fn try_mem_write(
//     addr: usize,
//     data: &[u8],
//     registers: OwnershipRegisters,
//     memory: &mut VmMemory,
// ) -> Result<(), RuntimeError> {
//     let ax = addr.checked_add(data.len()).ok_or(PanicReason::ArithmeticOverflow)?;

//     let range = (ax <= VM_MAX_RAM as usize)
//         .then(|| MemoryRange::new(addr as Word, data.len() as Word))
//         .ok_or(PanicReason::MemoryOverflow)?;

//     registers
//         .has_ownership_range(&range)
//         .then(|| {
//             memory.get_mut(addr..ax)?.copy_from_slice(data);
//             Some(())
//         })
//         .flatten()
//         .ok_or_else(|| PanicReason::MemoryOwnership.into())
// }

// pub(crate) fn try_zeroize(
//     addr: usize,
//     len: usize,
//     registers: OwnershipRegisters,
//     memory: &mut VmMemory,
// ) -> Result<(), RuntimeError> {
//     let ax = addr.checked_add(len).ok_or(PanicReason::ArithmeticOverflow)?;

//     let range = (ax <= VM_MAX_RAM as usize)
//         .then(|| MemoryRange::new(addr as Word, len as Word))
//         .ok_or(PanicReason::MemoryOverflow)?;

//     registers
//         .has_ownership_range(&range)
//         .then(|| {
//             memory[addr..].iter_mut().take(len).for_each(|m| *m = 0);
//         })
//         .ok_or_else(|| PanicReason::MemoryOwnership.into())
// }

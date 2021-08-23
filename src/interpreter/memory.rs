use super::{ExecuteError, Interpreter};
use crate::consts::*;

use fuel_asm::{RegisterId, Word};
use fuel_tx::Bytes8;

use std::{ptr, slice};

mod range;

pub use range::MemoryRange;

impl<S> Interpreter<S> {
    /// Copy `data` into `addr[..|data|[`
    ///
    /// Check for overflow and memory ownership
    ///
    /// # Panics
    ///
    /// Will panic if data overlaps with `addr[..|data|[`
    pub(crate) fn try_mem_write(&mut self, addr: usize, data: &[u8]) -> Result<(), ExecuteError> {
        addr.checked_add(data.len())
            .ok_or(ExecuteError::ArithmeticOverflow)
            .and_then(|ax| {
                (ax <= VM_MAX_RAM as usize)
                    .then(|| MemoryRange::new(addr as Word, data.len() as Word))
                    .ok_or(ExecuteError::MemoryOverflow)
            })
            .and_then(|range| {
                self.has_ownership_range(&range)
                    .then(|| {
                        let src = data.as_ptr();
                        let dst = &mut self.memory[addr] as *mut u8;

                        unsafe {
                            ptr::copy_nonoverlapping(src, dst, data.len());
                        }
                    })
                    .ok_or(ExecuteError::MemoryOwnership)
            })
    }

    pub(crate) fn try_zeroize(&mut self, addr: usize, len: usize) -> Result<(), ExecuteError> {
        addr.checked_add(len)
            .ok_or(ExecuteError::ArithmeticOverflow)
            .and_then(|ax| {
                (ax <= VM_MAX_RAM as usize)
                    .then(|| MemoryRange::new(addr as Word, len as Word))
                    .ok_or(ExecuteError::MemoryOverflow)
            })
            .and_then(|range| {
                self.has_ownership_range(&range)
                    .then(|| {
                        (&mut self.memory[addr..]).iter_mut().take(len).for_each(|m| *m = 0);
                    })
                    .ok_or(ExecuteError::MemoryOwnership)
            })
    }

    /// Grant ownership of the range `[a..ab[`
    pub(crate) fn has_ownership_range(&self, range: &MemoryRange) -> bool {
        let (a, ab) = range.boundaries(self);

        let a_is_stack = a < self.registers[REG_SP];
        let a_is_heap = a > self.registers[REG_HP];

        let ab_is_stack = ab <= self.registers[REG_SP];
        let ab_is_heap = ab >= self.registers[REG_HP];

        a < ab
            && (a_is_stack && ab_is_stack && self.has_ownership_stack(a) && self.has_ownership_stack_exclusive(ab)
                || a_is_heap && ab_is_heap && self.has_ownership_heap(a) && self.has_ownership_heap_exclusive(ab))
    }

    pub(crate) const fn has_ownership_stack(&self, a: Word) -> bool {
        a <= VM_MAX_RAM && self.registers[REG_SSP] <= a && a < self.registers[REG_SP]
    }

    pub(crate) const fn has_ownership_stack_exclusive(&self, a: Word) -> bool {
        a <= VM_MAX_RAM && self.registers[REG_SSP] <= a && a <= self.registers[REG_SP]
    }

    pub(crate) fn has_ownership_heap(&self, a: Word) -> bool {
        // TODO implement fp->hp and (addr, size) validations
        // fp->hp
        // it means $hp from the previous context, i.e. what's saved in the
        // "Saved registers from previous context" of the call frame at
        // $fp`
        let external = self.is_external_context();

        self.registers[REG_HP] < a
            && (external && a < VM_MAX_RAM
                || !external && a <= self.frames.last().map(|frame| frame.registers()[REG_HP]).unwrap_or(0))
    }

    pub(crate) fn has_ownership_heap_exclusive(&self, a: Word) -> bool {
        // TODO reflect the pending changes from `has_ownership_heap`
        let external = self.is_external_context();

        self.registers[REG_HP] < a
            && (external && a <= VM_MAX_RAM
                || !external && a <= self.frames.last().map(|frame| frame.registers()[REG_HP]).unwrap_or(0) + 1)
    }

    pub(crate) const fn is_stack_address(&self, a: Word) -> bool {
        a < self.registers[REG_SP]
    }

    pub(crate) fn extend_call_frame(&mut self, imm: Word) -> Result<(), ExecuteError> {
        if self.registers[REG_SP] > VM_MAX_RAM - imm {
            return Err(ExecuteError::StackOverflow);
        }

        self.registers[REG_SP] += imm;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn shrink_call_frame(&mut self, imm: Word) -> Result<(), ExecuteError> {
        if imm > self.registers[REG_SP] || self.registers[REG_SSP] > self.registers[REG_SP] - imm {
            return Err(ExecuteError::StackShrinkViolation);
        }

        self.registers[REG_SP] -= imm;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn load_byte(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), ExecuteError> {
        if b > VM_MAX_RAM || c > VM_MAX_RAM || b + c > VM_MAX_RAM {
            return Err(ExecuteError::MemoryOverflow);
        }

        self.registers[ra] = self.memory[(b + c) as usize] as Word;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn load_word(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), ExecuteError> {
        if c > VM_MAX_RAM - 8 || b > VM_MAX_RAM - 8 - c * 8 {
            return Err(ExecuteError::MemoryOverflow);
        }

        let (b, c) = (b as usize, c as usize);

        // C is expressed in words
        let bc = b + c * 8;

        // Safety: memory bounds checked by interpreter
        let word = unsafe { Bytes8::from_slice_unchecked(&self.memory[bc..bc + 8]) };
        self.registers[ra] = Word::from_be_bytes(word.into());

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn store_byte(&mut self, a: Word, b: Word, c: Word) -> Result<(), ExecuteError> {
        if c > VM_MAX_RAM || a > VM_MAX_RAM - c {
            return Err(ExecuteError::MemoryOverflow);
        }

        let ac = a + c;
        if !(self.has_ownership_stack(ac) || self.has_ownership_heap(ac)) {
            return Err(ExecuteError::MemoryOwnership);
        }

        self.memory[ac as usize] = b as u8;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn store_word(&mut self, a: Word, b: Word, c: Word) -> Result<(), ExecuteError> {
        if c > VM_MAX_RAM / 8 || a >= VM_MAX_RAM - c / 8 {
            return Err(ExecuteError::MemoryOverflow);
        }

        // C is expressed in words
        let ac = a + c * 8;

        self.try_mem_write(ac as usize, &b.to_be_bytes())?;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn malloc(&mut self, a: Word) -> Result<(), ExecuteError> {
        if self.registers[REG_HP] - self.registers[REG_SP] < a {
            return Err(ExecuteError::MemoryOverflow);
        }

        self.registers[REG_HP] -= a;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn memclear(&mut self, a: Word, b: Word) -> Result<(), ExecuteError> {
        self.try_zeroize(a as usize, b as usize)?;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn memcopy(&mut self, a: Word, b: Word, c: Word) -> Result<(), ExecuteError> {
        if c > MEM_MAX_ACCESS_SIZE || a > VM_MAX_RAM - c || b > VM_MAX_RAM - c {
            return Err(ExecuteError::MemoryOverflow);
        }

        let (a, b, c) = (a as usize, b as usize, c as usize);

        let ac = a + c;
        let bc = b + c;

        if a <= b && b < ac || b <= a && a < bc {
            return Err(ExecuteError::MemoryOverlap);
        }

        // Safety: The pointers are granted to be aligned and overlap is checked
        let src = unsafe { slice::from_raw_parts(&self.memory[b] as *const u8, c) };

        self.try_mem_write(a, src)?;

        self.inc_pc();

        Ok(())
    }

    pub(crate) fn memeq(&mut self, ra: RegisterId, b: Word, c: Word, d: Word) -> Result<(), ExecuteError> {
        if d > MEM_MAX_ACCESS_SIZE || b > VM_MAX_RAM - d || c > VM_MAX_RAM - d {
            return Err(ExecuteError::MemoryOverflow);
        }

        let (b, c, d) = (b as usize, c as usize, d as usize);

        self.registers[ra] = (self.memory[b..b + d] == self.memory[c..c + d]) as Word;

        self.inc_pc();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::consts::*;
    use crate::prelude::*;

    #[test]
    fn memcopy() {
        let storage = MemoryStorage::default();
        let mut vm = Interpreter::with_storage(storage);
        vm.init(Transaction::default()).expect("Failed to init VM");

        let alloc = 1024;

        // r[0x10] := 1024
        vm.execute(Opcode::ADDI(0x10, REG_ZERO, alloc)).unwrap();
        vm.execute(Opcode::ALOC(0x10)).unwrap();

        // r[0x20] := 128
        vm.execute(Opcode::ADDI(0x20, 0x20, 128)).unwrap();

        for i in 0..alloc {
            vm.execute(Opcode::ADDI(0x21, REG_ZERO, i)).unwrap();
            vm.execute(Opcode::SB(REG_HP, 0x21, (i + 1) as Immediate12)).unwrap();
        }

        // r[0x23] := m[$hp, 0x20] == m[0x12, 0x20]
        vm.execute(Opcode::MEQ(0x23, REG_HP, 0x12, 0x20)).unwrap();
        assert_eq!(0, vm.registers()[0x23]);

        // r[0x12] := $hp + r[0x20]
        vm.execute(Opcode::ADD(0x12, REG_HP, 0x20)).unwrap();
        vm.execute(Opcode::ADD(0x12, REG_ONE, 0x12)).unwrap();

        // Test ownership
        vm.execute(Opcode::ADD(0x30, REG_HP, REG_ONE)).unwrap();
        vm.execute(Opcode::MCP(0x30, 0x12, 0x20)).unwrap();

        // r[0x23] := m[0x30, 0x20] == m[0x12, 0x20]
        vm.execute(Opcode::MEQ(0x23, 0x30, 0x12, 0x20)).unwrap();
        assert_eq!(1, vm.registers()[0x23]);

        // Assert ownership
        vm.execute(Opcode::SUBI(0x24, REG_HP, 1)).unwrap();
        let ownership_violated = vm.execute(Opcode::MCP(0x24, 0x12, 0x20));
        assert!(ownership_violated.is_err());

        // Assert no panic on overlapping
        vm.execute(Opcode::SUBI(0x25, 0x12, 1)).unwrap();
        let overlapping = vm.execute(Opcode::MCP(REG_HP, 0x25, 0x20));
        assert!(overlapping.is_err());
    }

    #[test]
    fn memrange() {
        let m = MemoryRange::from(..1024);
        let m_p = MemoryRange::new(0, 1024);
        assert_eq!(m, m_p);

        let storage = MemoryStorage::default();
        let mut vm = Interpreter::with_storage(storage);
        vm.init(Transaction::default()).expect("Failed to init VM");

        let bytes = 1024;
        vm.execute(Opcode::ADDI(0x10, REG_ZERO, bytes as Immediate12)).unwrap();
        vm.execute(Opcode::ALOC(0x10)).unwrap();

        let m = MemoryRange::new(vm.registers()[REG_HP], bytes);
        assert!(!vm.has_ownership_range(&m));

        let m = MemoryRange::new(vm.registers()[REG_HP] + 1, bytes);
        assert!(vm.has_ownership_range(&m));

        let m = MemoryRange::new(vm.registers()[REG_HP] + 1, bytes + 1);
        assert!(!vm.has_ownership_range(&m));

        let m = MemoryRange::new(0, bytes).to_heap(&vm);
        assert!(vm.has_ownership_range(&m));

        let m = MemoryRange::new(0, bytes + 1).to_heap(&vm);
        assert!(!vm.has_ownership_range(&m));
    }

    #[test]
    fn stack_alloc_ownership() {
        let storage = MemoryStorage::default();
        let mut vm = Interpreter::with_storage(storage);
        vm.init(Transaction::default()).expect("Failed to init VM");

        vm.execute(Opcode::MOVE(0x10, REG_SP)).unwrap();
        vm.execute(Opcode::CFEI(2)).unwrap();

        // Assert allocated stack is writable
        vm.execute(Opcode::MCLI(0x10, 2)).unwrap();
    }
}

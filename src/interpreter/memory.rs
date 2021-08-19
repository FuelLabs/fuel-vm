use super::{ExecuteError, Interpreter};
use crate::consts::*;

use fuel_asm::{RegisterId, Word};

use std::convert::TryFrom;
use std::ptr;

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
    pub(crate) fn try_mem_write(&mut self, addr: Word, data: &[u8]) -> Result<(), ExecuteError> {
        addr.checked_add(data.len() as Word)
            .ok_or(ExecuteError::ArithmeticOverflow)
            .and_then(|ax| {
                (ax <= VM_MAX_RAM)
                    .then(|| MemoryRange::new(addr, 32))
                    .ok_or(ExecuteError::MemoryOverflow)
            })
            .and_then(|range| {
                self.has_ownership_range(&range)
                    .then(|| {
                        let src = data.as_ptr();
                        let dst = &mut self.memory[addr as usize] as *mut u8;

                        unsafe {
                            ptr::copy_nonoverlapping(src, dst, data.len());
                        }
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

    pub(crate) fn stack_pointer_overflow(&mut self, f: fn(Word, Word) -> (Word, bool), v: Word) -> bool {
        let (result, overflow) = f(self.registers[REG_SP], v);

        if overflow || result > self.registers[REG_HP] {
            false
        } else {
            self.registers[REG_SP] = result;
            true
        }
    }

    pub(crate) fn load_byte(&mut self, ra: RegisterId, b: RegisterId, c: Word) -> bool {
        let bc = b.saturating_add(c as RegisterId);

        if bc >= VM_MAX_RAM as RegisterId {
            false
        } else {
            // TODO ensure the byte should be cast and not overwrite the
            // encoding
            self.registers[ra] = self.memory[bc] as Word;

            true
        }
    }

    pub(crate) fn load_word(&mut self, ra: RegisterId, b: Word, c: Word) -> bool {
        // C is expressed in words; mul by 8
        let (bc, overflow) = b.overflowing_add(c * 8);
        let (bcw, of) = bc.overflowing_add(8);
        let overflow = overflow || of;

        let bc = bc as usize;
        let bcw = bcw as usize;

        if overflow || bcw > VM_MAX_RAM as RegisterId {
            false
        } else {
            // Safe conversion of sized slice
            self.registers[ra] = <[u8; 8]>::try_from(&self.memory[bc..bcw])
                .map(Word::from_be_bytes)
                .unwrap_or_else(|_| unreachable!());

            true
        }
    }

    pub(crate) fn store_byte(&mut self, a: Word, b: Word, c: Word) -> bool {
        let (ac, overflow) = a.overflowing_add(c);

        if overflow || ac >= VM_MAX_RAM || !(self.has_ownership_stack(ac) || self.has_ownership_heap(ac)) {
            false
        } else {
            self.memory[ac as usize] = b as u8;

            true
        }
    }

    pub(crate) fn store_word(&mut self, a: Word, b: Word, c: Word) -> bool {
        // C is expressed in words; mul by 8
        let (ac, overflow) = a.overflowing_add(c * 8);
        let (acw, of) = ac.overflowing_add(8);
        let overflow = overflow || of;

        let range = MemoryRange::new(ac, 8);
        if overflow || acw > VM_MAX_RAM || !self.has_ownership_range(&range) {
            false
        } else {
            // TODO review if BE is intended
            self.memory[ac as usize..acw as usize].copy_from_slice(&b.to_be_bytes());

            true
        }
    }

    pub(crate) fn malloc(&mut self, a: Word) -> bool {
        let (result, overflow) = self.registers[REG_HP].overflowing_sub(a);

        if overflow || result < self.registers[REG_SP] {
            false
        } else {
            self.registers[REG_HP] = result;
            true
        }
    }

    pub(crate) fn memclear(&mut self, a: Word, b: Word) -> bool {
        let (ab, overflow) = a.overflowing_add(b);

        let range = MemoryRange::new(a, b);
        if overflow || ab > VM_MAX_RAM || b > MEM_MAX_ACCESS_SIZE || !self.has_ownership_range(&range) {
            false
        } else {
            // trivial compiler optimization for memset when best
            for i in &mut self.memory[a as usize..ab as usize] {
                *i = 0
            }
            true
        }
    }

    pub(crate) fn memcopy(&mut self, a: Word, b: Word, c: Word) -> bool {
        let (ac, overflow) = a.overflowing_add(c);
        let (bc, of) = b.overflowing_add(c);
        let overflow = overflow || of;

        let range = MemoryRange::new(a, c);
        if overflow
            || ac > VM_MAX_RAM
            || bc > VM_MAX_RAM
            || c > MEM_MAX_ACCESS_SIZE
            || a <= b && b < ac
            || b <= a && a < bc
            || !self.has_ownership_range(&range)
        {
            false
        } else {
            // The pointers are granted to be aligned so this is a safe
            // operation
            let src = &self.memory[b as usize] as *const u8;
            let dst = &mut self.memory[a as usize] as *mut u8;

            unsafe {
                ptr::copy_nonoverlapping(src, dst, c as usize);
            }

            true
        }
    }

    pub(crate) fn memeq(&mut self, ra: RegisterId, b: Word, c: Word, d: Word) -> bool {
        let (bd, overflow) = b.overflowing_add(d);
        let (cd, of) = c.overflowing_add(d);
        let overflow = overflow || of;

        if overflow || bd > VM_MAX_RAM || cd > VM_MAX_RAM || d > MEM_MAX_ACCESS_SIZE {
            false
        } else {
            self.registers[ra] = (self.memory[b as usize..bd as usize] == self.memory[c as usize..cd as usize]) as Word;

            true
        }
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

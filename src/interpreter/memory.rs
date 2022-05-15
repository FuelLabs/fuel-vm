use super::Interpreter;
use crate::consts::*;
use crate::error::RuntimeError;

use fuel_asm::PanicReason;
use fuel_types::{RegisterId, Word};

use std::{ops, ptr};

#[derive(Debug, Clone, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Memory range representation for the VM.
///
/// `start` is inclusive, and `end` is exclusive.
pub struct MemoryRange {
    start: ops::Bound<Word>,
    end: ops::Bound<Word>,
    len: Word,
}

impl MemoryRange {
    /// Create a new memory range represented as `[address, address + size[`.
    pub const fn new(address: Word, size: Word) -> Self {
        let start = ops::Bound::Included(address);
        let end = ops::Bound::Excluded(address.saturating_add(size));
        let len = size;

        Self { start, end, len }
    }

    /// Beginning of the memory range.
    pub const fn start(&self) -> Word {
        use ops::Bound::*;

        match self.start {
            Included(start) => start,
            Excluded(start) => start.saturating_add(1),
            Unbounded => 0,
        }
    }

    /// End of the memory range.
    pub const fn end(&self) -> Word {
        use ops::Bound::*;

        match self.end {
            Included(end) => end.saturating_add(1),
            Excluded(end) => end,
            Unbounded => VM_MAX_RAM,
        }
    }

    /// Bytes count of this memory range.
    pub const fn len(&self) -> Word {
        self.len
    }

    /// Return `true` if the length is `0`.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the boundaries of the slice with exclusive end `[a, b[`
    ///
    /// Remap the unbound boundaries to stack or heap when applicable.
    pub const fn boundaries<S>(&self, vm: &Interpreter<S>) -> (Word, Word) {
        use ops::Bound::*;

        let stack = vm.registers()[REG_SP];
        let heap = vm.registers()[REG_HP].saturating_add(1);
        match (self.start, self.end) {
            (Included(start), Included(end)) => (start, end.saturating_add(1)),
            (Included(start), Excluded(end)) => (start, end),
            (Excluded(start), Included(end)) => (start.saturating_add(1), end.saturating_add(1)),
            (Excluded(start), Excluded(end)) => (start.saturating_add(1), end),
            (Unbounded, Unbounded) => (0, VM_MAX_RAM),

            (Included(start), Unbounded) if vm.is_stack_address(start) => (start, stack),
            (Included(start), Unbounded) => (start, VM_MAX_RAM),

            (Excluded(start), Unbounded) if vm.is_stack_address(start) => (start.saturating_add(1), stack),
            (Excluded(start), Unbounded) => (start.saturating_add(1), VM_MAX_RAM),

            (Unbounded, Included(end)) if vm.is_stack_address(end) => (0, end.saturating_add(1)),
            (Unbounded, Included(end)) => (heap, end),

            (Unbounded, Excluded(end)) if vm.is_stack_address(end) => (0, end),
            (Unbounded, Excluded(end)) => (heap, end),
        }
    }

    /// Return an owned memory slice with a relative address to the heap space
    /// defined in `r[$hp]`
    pub const fn to_heap<S>(mut self, vm: &Interpreter<S>) -> Self {
        use ops::Bound::*;

        let heap = vm.registers()[REG_HP].saturating_add(1);

        self.start = match self.start {
            Included(start) => Included(heap.saturating_add(start)),
            Excluded(start) => Included(heap.saturating_add(start).saturating_add(1)),
            Unbounded => Included(heap),
        };

        self.end = match self.end {
            Included(end) => Excluded(heap.saturating_add(end).saturating_add(1)),
            Excluded(end) => Excluded(heap.saturating_add(end)),
            Unbounded => Excluded(VM_MAX_RAM),
        };

        let (start, end) = self.boundaries(vm);
        self.len = end.saturating_sub(start);

        self
    }
}

impl<R> From<R> for MemoryRange
where
    R: ops::RangeBounds<Word>,
{
    fn from(range: R) -> MemoryRange {
        use ops::Bound::*;
        // Owned bounds are unstable
        // https://github.com/rust-lang/rust/issues/61356

        let (start, s) = match range.start_bound() {
            Included(start) => (Included(*start), *start),
            Excluded(start) => (Included(start.saturating_add(1)), start.saturating_add(1)),
            Unbounded => (Unbounded, 0),
        };

        let (end, e) = match range.end_bound() {
            Included(end) => (Excluded(end.saturating_add(1)), end.saturating_add(1)),
            Excluded(end) => (Excluded(*end), *end),
            Unbounded => (Unbounded, VM_MAX_RAM),
        };

        let len = e.saturating_sub(s);

        Self { start, end, len }
    }
}

// Memory bounds must be manually checked and cannot follow general PartialEq
// rules
impl PartialEq for MemoryRange {
    fn eq(&self, other: &MemoryRange) -> bool {
        use ops::Bound::*;

        let start = match (self.start, other.start) {
            (Included(a), Included(b)) => a == b,
            (Included(a), Excluded(b)) => a == b.saturating_add(1),
            (Included(a), Unbounded) => a == 0,
            (Excluded(a), Included(b)) => a.saturating_add(1) == b,
            (Excluded(a), Excluded(b)) => a == b,
            (Excluded(a), Unbounded) => a == 0,
            (Unbounded, Included(b)) => b == 0,
            (Unbounded, Excluded(b)) => b == 0,
            (Unbounded, Unbounded) => true,
        };

        let end = match (self.end, other.end) {
            (Included(a), Included(b)) => a == b,
            (Included(a), Excluded(b)) => a == b.saturating_sub(1),
            (Included(a), Unbounded) => a == VM_MAX_RAM - 1,
            (Excluded(a), Included(b)) => a.saturating_sub(1) == b,
            (Excluded(a), Excluded(b)) => a == b,
            (Excluded(a), Unbounded) => a == VM_MAX_RAM,
            (Unbounded, Included(b)) => b == VM_MAX_RAM - 1,
            (Unbounded, Excluded(b)) => b == VM_MAX_RAM,
            (Unbounded, Unbounded) => true,
        };

        start && end
    }
}

impl<S> Interpreter<S> {
    /// Copy `data` into `addr[..|data|[`
    ///
    /// Check for overflow and memory ownership
    ///
    /// # Panics
    ///
    /// Will panic if data overlaps with `addr[..|data|[`
    pub(crate) fn try_mem_write(&mut self, addr: usize, data: &[u8]) -> Result<(), RuntimeError> {
        let ax = addr.checked_add(data.len()).ok_or(PanicReason::ArithmeticOverflow)?;

        let range = (ax <= VM_MAX_RAM as usize)
            .then(|| MemoryRange::new(addr as Word, 32))
            .ok_or(PanicReason::MemoryOverflow)?;

        self.has_ownership_range(&range)
            .then(|| {
                let src = data.as_ptr();
                let dst = &mut self.memory[addr] as *mut u8;

                unsafe {
                    ptr::copy_nonoverlapping(src, dst, data.len());
                }
            })
            .ok_or(PanicReason::MemoryOwnership.into())
    }

    pub(crate) fn try_zeroize(&mut self, addr: usize, len: usize) -> Result<(), RuntimeError> {
        let ax = addr.checked_add(len).ok_or(PanicReason::ArithmeticOverflow)?;

        let range = (ax <= VM_MAX_RAM as usize)
            .then(|| MemoryRange::new(addr as Word, 32))
            .ok_or(PanicReason::MemoryOverflow)?;

        self.has_ownership_range(&range)
            .then(|| {
                (&mut self.memory[addr..]).iter_mut().take(len).for_each(|m| *m = 0);
            })
            .ok_or(PanicReason::MemoryOwnership.into())
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

    pub(crate) fn stack_pointer_overflow<F>(&mut self, f: F, v: Word) -> Result<(), RuntimeError>
    where
        F: FnOnce(Word, Word) -> (Word, bool),
    {
        let (result, overflow) = f(self.registers[REG_SP], v);

        if overflow || result > self.registers[REG_HP] {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            self.registers[REG_SP] = result;

            self.inc_pc()
        }
    }

    pub(crate) fn load_byte(&mut self, ra: RegisterId, b: RegisterId, c: Word) -> Result<(), RuntimeError> {
        let bc = b.saturating_add(c as RegisterId);

        if bc >= VM_MAX_RAM as RegisterId {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            self.registers[ra] = self.memory[bc] as Word;

            self.inc_pc()
        }
    }

    pub(crate) fn load_word(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), RuntimeError> {
        // C is expressed in words; mul by 8
        let (bc, overflow) = b.overflowing_add(c * 8);
        let (bcw, of) = bc.overflowing_add(8);
        let overflow = overflow || of;

        let bc = bc as usize;
        let bcw = bcw as usize;

        if overflow || bcw > VM_MAX_RAM as RegisterId {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            // Safe conversion of sized slice
            self.registers[ra] = <[u8; 8]>::try_from(&self.memory[bc..bcw])
                .map(Word::from_be_bytes)
                .unwrap_or_else(|_| unreachable!());

            self.inc_pc()
        }
    }

    pub(crate) fn store_byte(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let (ac, overflow) = a.overflowing_add(c);

        if overflow || ac >= VM_MAX_RAM || !(self.has_ownership_stack(ac) || self.has_ownership_heap(ac)) {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            self.memory[ac as usize] = b as u8;

            self.inc_pc()
        }
    }

    pub(crate) fn store_word(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        // C is expressed in words; mul by 8
        let (ac, overflow) = a.overflowing_add(c * 8);
        let (acw, of) = ac.overflowing_add(8);
        let overflow = overflow || of;

        let range = MemoryRange::new(ac, 8);
        if overflow || acw > VM_MAX_RAM || !self.has_ownership_range(&range) {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            self.memory[ac as usize..acw as usize].copy_from_slice(&b.to_be_bytes());

            self.inc_pc()
        }
    }

    pub(crate) fn malloc(&mut self, a: Word) -> Result<(), RuntimeError> {
        let (result, overflow) = self.registers[REG_HP].overflowing_sub(a);

        if overflow || result < self.registers[REG_SP] {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            self.registers[REG_HP] = result;

            self.inc_pc()
        }
    }

    pub(crate) fn memclear(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        let (ab, overflow) = a.overflowing_add(b);

        let range = MemoryRange::new(a, b);
        if overflow || ab > VM_MAX_RAM || b > MEM_MAX_ACCESS_SIZE || !self.has_ownership_range(&range) {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            // trivial compiler optimization for memset
            for i in &mut self.memory[a as usize..ab as usize] {
                *i = 0
            }

            self.inc_pc()
        }
    }

    pub(crate) fn memcopy(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
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
            Err(PanicReason::MemoryOverflow.into())
        } else {
            // The pointers are granted to be aligned so this is a safe
            // operation
            let src = &self.memory[b as usize] as *const u8;
            let dst = &mut self.memory[a as usize] as *mut u8;

            unsafe {
                ptr::copy_nonoverlapping(src, dst, c as usize);
            }

            self.inc_pc()
        }
    }

    pub(crate) fn memeq(&mut self, ra: RegisterId, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let (bd, overflow) = b.overflowing_add(d);
        let (cd, of) = c.overflowing_add(d);
        let overflow = overflow || of;

        if overflow || bd > VM_MAX_RAM || cd > VM_MAX_RAM || d > MEM_MAX_ACCESS_SIZE {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            self.registers[ra] = (self.memory[b as usize..bd as usize] == self.memory[c as usize..cd as usize]) as Word;

            self.inc_pc()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::consts::*;
    use crate::prelude::*;

    #[test]
    fn memcopy() {
        let mut vm = Interpreter::with_memory_storage();
        vm.init(Transaction::default()).expect("Failed to init VM");

        let alloc = 1024;

        // r[0x10] := 1024
        vm.instruction(Opcode::ADDI(0x10, REG_ZERO, alloc).into()).unwrap();
        vm.instruction(Opcode::ALOC(0x10).into()).unwrap();

        // r[0x20] := 128
        vm.instruction(Opcode::ADDI(0x20, 0x20, 128).into()).unwrap();

        for i in 0..alloc {
            vm.instruction(Opcode::ADDI(0x21, REG_ZERO, i).into()).unwrap();
            vm.instruction(Opcode::SB(REG_HP, 0x21, (i + 1) as Immediate12).into())
                .unwrap();
        }

        // r[0x23] := m[$hp, 0x20] == m[0x12, 0x20]
        vm.instruction(Opcode::MEQ(0x23, REG_HP, 0x12, 0x20).into()).unwrap();
        assert_eq!(0, vm.registers()[0x23]);

        // r[0x12] := $hp + r[0x20]
        vm.instruction(Opcode::ADD(0x12, REG_HP, 0x20).into()).unwrap();
        vm.instruction(Opcode::ADD(0x12, REG_ONE, 0x12).into()).unwrap();

        // Test ownership
        vm.instruction(Opcode::ADD(0x30, REG_HP, REG_ONE).into()).unwrap();
        vm.instruction(Opcode::MCP(0x30, 0x12, 0x20).into()).unwrap();

        // r[0x23] := m[0x30, 0x20] == m[0x12, 0x20]
        vm.instruction(Opcode::MEQ(0x23, 0x30, 0x12, 0x20).into()).unwrap();
        assert_eq!(1, vm.registers()[0x23]);

        // Assert ownership
        vm.instruction(Opcode::SUBI(0x24, REG_HP, 1).into()).unwrap();
        let ownership_violated = vm.instruction(Opcode::MCP(0x24, 0x12, 0x20).into());
        assert!(ownership_violated.is_err());

        // Assert no panic on overlapping
        vm.instruction(Opcode::SUBI(0x25, 0x12, 1).into()).unwrap();
        let overlapping = vm.instruction(Opcode::MCP(REG_HP, 0x25, 0x20).into());
        assert!(overlapping.is_err());
    }

    #[test]
    fn memrange() {
        let m = MemoryRange::from(..1024);
        let m_p = MemoryRange::new(0, 1024);
        assert_eq!(m, m_p);

        let mut vm = Interpreter::with_memory_storage();
        vm.init(Transaction::default()).expect("Failed to init VM");

        let bytes = 1024;
        vm.instruction(Opcode::ADDI(0x10, REG_ZERO, bytes as Immediate12).into())
            .unwrap();
        vm.instruction(Opcode::ALOC(0x10).into()).unwrap();

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
        let mut vm = Interpreter::with_memory_storage();
        vm.init(Transaction::default()).expect("Failed to init VM");

        vm.instruction(Opcode::MOVE(0x10, REG_SP).into()).unwrap();
        vm.instruction(Opcode::CFEI(2).into()).unwrap();

        // Assert allocated stack is writable
        vm.instruction(Opcode::MCLI(0x10, 2).into()).unwrap();
    }
}

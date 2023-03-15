use super::internal::inc_pc;
use super::{ExecutableTransaction, Interpreter};
use crate::constraints::reg_key::*;
use crate::error::RuntimeError;
use crate::{consts::*, context::Context};

use fuel_asm::{PanicReason, RegId};
use fuel_types::{RegisterId, Word};

use std::ops::Range;
use std::ops;

pub type Memory<const SIZE: usize> = Box<[u8; SIZE]>;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod allocation_tests;
#[allow(clippy::derive_hash_xor_eq)]
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

impl Default for MemoryRange {
    fn default() -> Self {
        Self {
            start: ops::Bound::Included(0),
            end: ops::Bound::Excluded(0),
            len: 0,
        }
    }
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
    pub fn boundaries(&self, registers: &OwnershipRegisters) -> (Word, Word) {
        use ops::Bound::*;

        let stack = registers.sp;
        let heap = registers.hp.saturating_add(1);
        match (self.start, self.end) {
            (Included(start), Included(end)) => (start, end.saturating_add(1)),
            (Included(start), Excluded(end)) => (start, end),
            (Excluded(start), Included(end)) => (start.saturating_add(1), end.saturating_add(1)),
            (Excluded(start), Excluded(end)) => (start.saturating_add(1), end),
            (Unbounded, Unbounded) => (0, VM_MAX_RAM),

            (Included(start), Unbounded) if is_stack_address(&registers.sp, start) => (start, stack),
            (Included(start), Unbounded) => (start, VM_MAX_RAM),

            (Excluded(start), Unbounded) if is_stack_address(&registers.sp, start) => (start.saturating_add(1), stack),
            (Excluded(start), Unbounded) => (start.saturating_add(1), VM_MAX_RAM),

            (Unbounded, Included(end)) if is_stack_address(&registers.sp, end) => (0, end.saturating_add(1)),
            (Unbounded, Included(end)) => (heap, end),

            (Unbounded, Excluded(end)) if is_stack_address(&registers.sp, end) => (0, end),
            (Unbounded, Excluded(end)) => (heap, end),
        }
    }

    /// Return an owned memory slice with a relative address to the heap space
    /// defined in `r[$hp]`
    pub fn to_heap<S, Tx>(mut self, vm: &Interpreter<S, Tx>) -> Self
    where
        Tx: ExecutableTransaction,
    {
        use ops::Bound::*;

        let heap = vm.registers()[RegId::HP];

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

        let (start, end) = self.boundaries(&OwnershipRegisters::new(vm));
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

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    /// Return the registers used to determine ownership.
    pub(crate) fn ownership_registers(&self) -> OwnershipRegisters {
        OwnershipRegisters::new(self)
    }

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

pub(crate) fn load_byte(
    memory: &[u8; MEM_SIZE],
    pc: RegMut<PC>,
    result: &mut Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
    let bc = b.saturating_add(c) as usize;

    if bc >= VM_MAX_RAM as RegisterId {
        Err(PanicReason::MemoryOverflow.into())
    } else {
        *result = memory[bc] as Word;

        inc_pc(pc)
    }
}

pub(crate) fn load_word(
    memory: &[u8; MEM_SIZE],
    pc: RegMut<PC>,
    result: &mut Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
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
        *result = <[u8; 8]>::try_from(&memory[bc..bcw])
            .map(Word::from_be_bytes)
            .unwrap_or_else(|_| unreachable!());

        inc_pc(pc)
    }
}

pub(crate) fn store_byte(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
    let (ac, overflow) = a.overflowing_add(c);
    let range = ac..(ac + 1);
    if overflow || ac >= VM_MAX_RAM || !(owner.has_ownership_stack(&range) || owner.has_ownership_heap(&range)) {
        Err(PanicReason::MemoryOverflow.into())
    } else {
        memory[ac as usize] = b as u8;

        inc_pc(pc)
    }
}

pub(crate) fn store_word(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
    // C is expressed in words; mul by 8
    let (ac, overflow) = a.overflowing_add(c * 8);
    let (acw, of) = ac.overflowing_add(8);
    let overflow = overflow || of;

    let range = MemoryRange::new(ac, 8);
    if overflow || acw > VM_MAX_RAM || !owner.has_ownership_range(&range) {
        Err(PanicReason::MemoryOverflow.into())
    } else {
        memory[ac as usize..acw as usize].copy_from_slice(&b.to_be_bytes());

        inc_pc(pc)
    }
}

pub(crate) fn malloc(mut hp: RegMut<HP>, sp: Reg<SP>, pc: RegMut<PC>, a: Word) -> Result<(), RuntimeError> {
    let (result, overflow) = hp.overflowing_sub(a);

    if overflow || result < *sp {
        Err(PanicReason::MemoryOverflow.into())
    } else {
        *hp = result;

        inc_pc(pc)
    }
}

pub(crate) fn memclear(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
) -> Result<(), RuntimeError> {
    let (ab, overflow) = a.overflowing_add(b);

    let range = MemoryRange::new(a, b);
    if overflow || ab > VM_MAX_RAM || b > MEM_MAX_ACCESS_SIZE || !owner.has_ownership_range(&range) {
        Err(PanicReason::MemoryOverflow.into())
    } else {
        // trivial compiler optimization for memset
        for i in &mut memory[a as usize..ab as usize] {
            *i = 0
        }

        inc_pc(pc)
    }
}

pub(crate) fn memcopy(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
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
        || a < bc && bc <= ac
        || b < ac && ac <= bc
        || !owner.has_ownership_range(&range)
    {
        Err(PanicReason::MemoryOverflow.into())
    } else {
        if a <= b {
            let (dst, src) = memory.split_at_mut(b as usize);
            dst[a as usize..ac as usize].copy_from_slice(&src[..c as usize]);
        } else {
            let (src, dst) = memory.split_at_mut(a as usize);
            dst[..c as usize].copy_from_slice(&src[b as usize..bc as usize]);
        }

        inc_pc(pc)
    }
}

pub(crate) fn memeq(
    memory: &mut [u8; MEM_SIZE],
    result: &mut Word,
    pc: RegMut<PC>,
    b: Word,
    c: Word,
    d: Word,
) -> Result<(), RuntimeError> {
    let (bd, overflow) = b.overflowing_add(d);
    let (cd, of) = c.overflowing_add(d);
    let overflow = overflow || of;

    if overflow || bd > VM_MAX_RAM || cd > VM_MAX_RAM || d > MEM_MAX_ACCESS_SIZE {
        Err(PanicReason::MemoryOverflow.into())
    } else {
        *result = (memory[b as usize..bd as usize] == memory[c as usize..cd as usize]) as Word;

        inc_pc(pc)
    }
}

pub(crate) const fn is_stack_address(sp: &u64, a: Word) -> bool {
    a < *sp
}

pub struct OwnershipRegisters {
    pub(crate) sp: u64,
    pub(crate) ssp: u64,
    pub(crate) hp: u64,
    pub(crate) prev_hp: u64,
    pub(crate) context: Context,
}

impl OwnershipRegisters {
    pub(crate) fn new<S, Tx>(vm: &Interpreter<S, Tx>) -> Self {
        OwnershipRegisters {
            sp: vm.registers[RegId::SP],
            ssp: vm.registers[RegId::SSP],
            hp: vm.registers[RegId::HP],
            prev_hp: vm.frames.last().map(|frame| frame.registers()[RegId::HP]).unwrap_or(0),
            context: vm.context.clone(),
        }
    }
    pub(crate) fn has_ownership_range(&self, range: &MemoryRange) -> bool {
        let (start_incl, end_excl) = range.boundaries(self);

        let range = start_incl..end_excl;

        if range.is_empty() {
            return false;
        }

        if (self.ssp..self.sp).contains(&start_incl) {
            return self.has_ownership_stack(&range);
        }

        self.has_ownership_heap(&range)
    }

    /// Zero-length range is never owned
    pub(crate) fn has_ownership_stack(&self, range: &Range<Word>) -> bool {
        if range.is_empty() {
            return false;
        }

        if range.end > VM_MAX_RAM {
            return false;
        }

        (self.ssp..self.sp).contains(&range.start) && (self.ssp..=self.sp).contains(&range.end)
    }

    /// Zero-length range is never owned
    pub(crate) fn has_ownership_heap(&self, range: &Range<Word>) -> bool {
        // TODO implement fp->hp and (addr, size) validations
        // fp->hp
        // it means $hp from the previous context, i.e. what's saved in the
        // "Saved registers from previous context" of the call frame at
        // $fp`
        if range.is_empty() {
            return false;
        }

        if range.start < self.hp {
            return false;
        }

        let heap_end = if self.context.is_external() {
            VM_MAX_RAM
        } else {
            self.prev_hp
        };

        self.hp != heap_end && range.end <= heap_end
    }
}

pub(crate) fn try_mem_write(
    addr: usize,
    data: &[u8],
    registers: OwnershipRegisters,
    memory: &mut [u8; MEM_SIZE],
) -> Result<(), RuntimeError> {
    let ax = addr.checked_add(data.len()).ok_or(PanicReason::ArithmeticOverflow)?;

    let range = (ax <= VM_MAX_RAM as usize)
        .then(|| MemoryRange::new(addr as Word, data.len() as Word))
        .ok_or(PanicReason::MemoryOverflow)?;

    registers
        .has_ownership_range(&range)
        .then(|| {
            memory.get_mut(addr..ax)?.copy_from_slice(data);
            Some(())
        })
        .flatten()
        .ok_or_else(|| PanicReason::MemoryOwnership.into())
}

pub(crate) fn try_zeroize(
    addr: usize,
    len: usize,
    registers: OwnershipRegisters,
    memory: &mut [u8; MEM_SIZE],
) -> Result<(), RuntimeError> {
    let ax = addr.checked_add(len).ok_or(PanicReason::ArithmeticOverflow)?;

    let range = (ax <= VM_MAX_RAM as usize)
        .then(|| MemoryRange::new(addr as Word, len as Word))
        .ok_or(PanicReason::MemoryOverflow)?;

    registers
        .has_ownership_range(&range)
        .then(|| {
            memory[addr..].iter_mut().take(len).for_each(|m| *m = 0);
        })
        .ok_or_else(|| PanicReason::MemoryOwnership.into())
}

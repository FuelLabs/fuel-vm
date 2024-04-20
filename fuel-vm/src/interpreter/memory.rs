#![cfg(feature = "alloc")]

use super::{
    internal::inc_pc,
    Interpreter,
};
use crate::{
    constraints::reg_key::*,
    consts::*,
    context::Context,
    error::SimpleResult,
};

use derivative::Derivative;
use fuel_asm::{
    Imm12,
    Imm24,
    PanicReason,
    RegId,
};
use fuel_types::{
    RegisterId,
    Word,
};

use core::ops::Range;

#[cfg(any(test, feature = "test-helpers"))]
use core::ops::{
    Index,
    IndexMut,
    RangeFrom,
    RangeTo,
};

use alloc::{
    vec,
    vec::Vec,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod impl_tests;

#[cfg(test)]
mod allocation_tests;

#[cfg(test)]
mod stack_tests;

/// Resize the heap to at least `new_len` bytes, filling the new space with zeros.
/// If `new_len` is less than the current length, the function does nothing.
/// The function may grow the size more than `new_len` to avoid frequent
/// reallocations.
fn reverse_resize_at_least(vec: &mut Vec<u8>, new_len: usize) {
    if vec.len() >= new_len {
        return
    }

    // To reduce small allocations, allocate at least 256 bytes at once.
    // After that, double the allocation every time.
    let cap = new_len.next_power_of_two().clamp(256, MEM_SIZE);
    let mut new_vec = Vec::new();
    new_vec.reserve_exact(cap);
    let prefix_zeroes = cap
        .checked_sub(vec.len())
        .expect("Attempting to resize impossibly large heap memory");
    new_vec.extend(core::iter::repeat(0).take(prefix_zeroes));
    new_vec.extend(vec.iter().copied());
    *vec = new_vec;
}

/// The memory of the VM, represented as stack and heap.
#[derive(Debug, Clone, PartialEq, Eq, Derivative)]
pub struct Memory {
    /// Stack. Grows upwards.
    #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))]
    stack: Vec<u8>,
    /// Heap. Grows downwards from MEM_SIZE.
    #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))]
    heap: Vec<u8>,
    /// Lowest allowed heap address, i.e. hp register value.
    /// This is needed since we can allocate extra heap for performance reasons.
    hp: usize,
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory {
    /// Create a new VM memory.
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            heap: Vec::new(),
            hp: MEM_SIZE,
        }
    }

    /// Offset of the heap section
    fn heap_offset(&self) -> usize {
        MEM_SIZE.saturating_sub(self.heap.len())
    }

    /// Returns a linear memory representation where stack is at the beginning and heap is
    /// at the end.
    pub fn into_linear_memory(self) -> Vec<u8> {
        let uninit_memory_size = MEM_SIZE
            .saturating_sub(self.stack.len())
            .saturating_sub(self.heap.len());
        let uninit_memory = vec![0u8; uninit_memory_size];
        let mut memory = self.stack;
        memory.extend(uninit_memory);
        memory.extend(self.heap);
        memory
    }

    /// Grows the stack to be at least `new_sp` bytes.
    pub fn grow_stack(&mut self, new_sp: Word) -> Result<(), PanicReason> {
        #[allow(clippy::cast_possible_truncation)] // Safety: MEM_SIZE is usize
        let new_sp = new_sp.min(MEM_SIZE as Word) as usize;
        if new_sp > self.stack.len() {
            if new_sp > self.hp {
                return Err(PanicReason::MemoryGrowthOverlap)
            }

            self.stack.resize(new_sp, 0);
        }
        Ok(())
    }

    /// Grows the heap to be at least `new_hp` bytes.
    pub fn grow_heap(&mut self, sp: Reg<SP>, new_hp: Word) -> Result<(), PanicReason> {
        let new_hp_word = new_hp.min(MEM_SIZE as Word);
        #[allow(clippy::cast_possible_truncation)] // Safety: MEM_SIZE is usize
        let new_hp = new_hp_word as usize;

        if new_hp_word < *sp {
            return Err(PanicReason::MemoryGrowthOverlap)
        }

        #[allow(clippy::arithmetic_side_effects)] // Safety: ensured above with min
        let new_len = MEM_SIZE - new_hp;

        // Expand the heap allocation
        reverse_resize_at_least(&mut self.heap, new_len);
        self.hp = new_hp;

        // If heap enters region where stack has been, truncate the stack
        self.stack.truncate(new_hp);

        Ok(())
    }

    /// Verify that the memory range is accessble and return it as a range.
    pub fn verify<A: ToAddr, B: ToAddr>(
        &self,
        addr: A,
        count: B,
    ) -> Result<MemoryRange, PanicReason> {
        let start = addr.to_addr()?;
        let len = count.to_addr()?;
        let end = start.saturating_add(len);
        if end > MEM_SIZE {
            return Err(PanicReason::MemoryOverflow)
        }

        if end <= self.stack.len() || start >= self.hp {
            Ok(MemoryRange(start..end))
        } else {
            Err(PanicReason::UninitalizedMemoryAccess)
        }
    }

    /// Verify a constant-sized memory range.
    pub fn verify_const<A: ToAddr, const C: usize>(
        &self,
        addr: A,
    ) -> Result<MemoryRange, PanicReason> {
        self.verify(addr, C)
    }

    /// Returns a reference to memory for reading, if possible.
    #[allow(clippy::arithmetic_side_effects)] // Safety: subtractions are checked
    pub fn read<A: ToAddr, C: ToAddr>(
        &self,
        addr: A,
        count: C,
    ) -> Result<&[u8], PanicReason> {
        let range = self.verify(addr, count)?;

        if range.end() <= self.stack.len() {
            Ok(&self.stack[range.usizes()])
        } else if range.start() >= self.heap_offset() {
            let start = range.start() - self.heap_offset();
            let end = range.end() - self.heap_offset();
            Ok(&self.heap[start..end])
        } else {
            unreachable!("Range was verified to be valid")
        }
    }

    /// Reads a constant-sized byte array from memory, if possible.
    pub fn read_bytes<A: ToAddr, const C: usize>(
        &self,
        at: A,
    ) -> Result<[u8; C], PanicReason> {
        let mut result = [0; C];
        result.copy_from_slice(self.read(at, C)?);
        Ok(result)
    }

    /// Gets write access to memory, if possible.
    /// Doesn't perform any ownership checks.
    #[allow(clippy::arithmetic_side_effects)] // Safety: subtractions are checked
    pub fn write_noownerchecks<A: ToAddr, B: ToAddr>(
        &mut self,
        addr: A,
        len: B,
    ) -> Result<&mut [u8], PanicReason> {
        let range = self.verify(addr, len)?;
        if range.end() <= self.stack.len() {
            Ok(&mut self.stack[range.usizes()])
        } else if range.start() >= self.heap_offset() {
            let start = range.start() - self.heap_offset();
            let end = range.end() - self.heap_offset();
            Ok(&mut self.heap[start..end])
        } else {
            unreachable!("Range was verified to be valid")
        }
    }

    /// Writes a constant-sized byte array to memory, if possible.
    /// Doesn't perform any ownership checks.
    pub fn write_bytes_noownerchecks<A: ToAddr, const C: usize>(
        &mut self,
        addr: A,
        data: [u8; C],
    ) -> Result<(), PanicReason> {
        self.write_noownerchecks(addr, C)?.copy_from_slice(&data);
        Ok(())
    }

    /// Checks that memory is writable and returns a mutable slice to it.
    pub fn write<A: ToAddr, C: ToAddr>(
        &mut self,
        owner: OwnershipRegisters,
        addr: A,
        len: C,
    ) -> Result<&mut [u8], PanicReason> {
        let range = self.verify(addr, len)?;
        owner.verify_ownership(&range.words())?;
        self.write_noownerchecks(range.start(), range.len())
    }

    /// Writes a constant-sized byte array to memory, checking for ownership.
    pub fn write_bytes<A: ToAddr, const C: usize>(
        &mut self,
        owner: OwnershipRegisters,
        addr: A,
        data: [u8; C],
    ) -> Result<(), PanicReason> {
        self.write(owner, addr, data.len())?.copy_from_slice(&data);
        Ok(())
    }

    /// Copies the memory from `src` to `dst`.
    #[inline]
    #[track_caller]
    pub fn memcopy_noownerchecks<A: ToAddr, B: ToAddr, C: ToAddr>(
        &mut self,
        dst: A,
        src: B,
        len: C,
    ) -> Result<(), PanicReason> {
        // TODO: Optimize

        let src = src.to_addr()?;
        let dst = dst.to_addr()?;
        let len = len.to_addr()?;

        let tmp = self.read(src, len)?.to_vec();
        self.write_noownerchecks(dst, len)?.copy_from_slice(&tmp);
        Ok(())
    }

    /// Memory access to the raw stack buffer.
    /// Note that for efficiency reasons this might not match sp value.
    #[cfg(any(test, feature = "test-helpers"))]
    pub fn stack_raw(&self) -> &[u8] {
        &self.stack
    }

    /// Memory access to the raw heap buffer.
    /// Note that for efficiency reasons this might not match hp value.
    #[cfg(any(test, feature = "test-helpers"))]
    pub fn heap_raw(&self) -> &[u8] {
        &self.heap
    }
}

#[cfg(feature = "test-helpers")]
impl From<Vec<u8>> for Memory {
    fn from(stack: Vec<u8>) -> Self {
        Self {
            stack,
            ..Self::new()
        }
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl Index<Range<usize>> for Memory {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        self.read(index.start, index.len())
            .expect("Memory range out of bounds")
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl Index<RangeFrom<usize>> for Memory {
    type Output = [u8];

    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self[index.start..MEM_SIZE]
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl Index<RangeTo<usize>> for Memory {
    type Output = [u8];

    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self[0..index.end]
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl IndexMut<Range<usize>> for Memory {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        self.write_noownerchecks(index.start, index.len())
            .expect("Memory range out of bounds")
    }
}

/// Used to handle `Word` to `usize` conversions for memory addresses,
/// as well as checking that the resulting value is withing the VM ram boundaries.
pub trait ToAddr {
    /// Converts a value to `usize` used for memory addresses.
    /// Returns `Err` with `MemoryOverflow` if the resulting value does't fit in the VM
    /// memory. This can be used for both addresses and offsets.
    fn to_addr(self) -> Result<usize, PanicReason>;
}

impl ToAddr for usize {
    fn to_addr(self) -> Result<usize, PanicReason> {
        if self > MEM_SIZE {
            return Err(PanicReason::MemoryOverflow)
        }
        Ok(self)
    }
}

impl ToAddr for Word {
    fn to_addr(self) -> Result<usize, PanicReason> {
        let value = usize::try_from(self).map_err(|_| PanicReason::MemoryOverflow)?;
        value.to_addr()
    }
}

#[cfg(feature = "test-helpers")]
/// Implemented for `i32` to allow integer literals. Panics on negative values.
impl ToAddr for i32 {
    fn to_addr(self) -> Result<usize, PanicReason> {
        if self < 0 {
            panic!("Negative memory address");
        }
        let value = usize::try_from(self).map_err(|_| PanicReason::MemoryOverflow)?;
        value.to_addr()
    }
}

/// A range of memory. No guarantees are made about validity of access.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryRange(Range<usize>);

impl MemoryRange {
    /// Create a new memory range. Cannot panic, but the range may be invalid.
    pub const fn new(start: usize, len: usize) -> Self {
        Self(start..start.saturating_add(len))
    }

    /// Start of the range.
    pub fn start(&self) -> usize {
        self.0.start
    }

    /// End of the range. One past the last byte.
    pub fn end(&self) -> usize {
        self.0.end
    }

    /// Is the range empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Length of the range.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the range as a `usize` range.
    pub fn usizes(&self) -> Range<usize> {
        self.0.clone()
    }

    /// Returns the range as a `Word` range.
    pub fn words(&self) -> Range<Word> {
        self.0.start as Word..self.0.end as Word
    }

    /// Splits range at given relative offset. Panics if offset > range length.
    pub fn split_at_offset(self, at: usize) -> (Self, Self) {
        let mid = self.0.start.saturating_add(at);
        assert!(mid <= self.0.end);
        (Self(self.0.start..mid), Self(mid..self.0.end))
    }
}

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal> {
    /// Return the registers used to determine ownership.
    pub(crate) fn ownership_registers(&self) -> OwnershipRegisters {
        OwnershipRegisters::new(self)
    }

    pub(crate) fn stack_pointer_overflow<F>(&mut self, f: F, v: Word) -> SimpleResult<()>
    where
        F: FnOnce(Word, Word) -> (Word, bool),
    {
        let (
            SystemRegisters {
                sp, ssp, hp, pc, ..
            },
            _,
        ) = split_registers(&mut self.registers);
        stack_pointer_overflow(sp, ssp.as_ref(), hp.as_ref(), pc, f, v, &mut self.memory)
    }

    pub(crate) fn push_selected_registers(
        &mut self,
        segment: ProgramRegistersSegment,
        bitmask: Imm24,
    ) -> SimpleResult<()> {
        let (
            SystemRegisters {
                sp, ssp, hp, pc, ..
            },
            program_regs,
        ) = split_registers(&mut self.registers);
        push_selected_registers(
            &mut self.memory,
            sp,
            ssp.as_ref(),
            hp.as_ref(),
            pc,
            &program_regs,
            segment,
            bitmask,
        )
    }

    pub(crate) fn pop_selected_registers(
        &mut self,
        segment: ProgramRegistersSegment,
        bitmask: Imm24,
    ) -> SimpleResult<()> {
        let (
            SystemRegisters {
                sp, ssp, hp, pc, ..
            },
            mut program_regs,
        ) = split_registers(&mut self.registers);
        pop_selected_registers(
            &mut self.memory,
            sp,
            ssp.as_ref(),
            hp.as_ref(),
            pc,
            &mut program_regs,
            segment,
            bitmask,
        )
    }

    pub(crate) fn load_byte(
        &mut self,
        ra: RegisterId,
        b: Word,
        c: Word,
    ) -> SimpleResult<()> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        load_byte(&self.memory, pc, result, b, c)
    }

    pub(crate) fn load_word(
        &mut self,
        ra: RegisterId,
        b: Word,
        c: Imm12,
    ) -> SimpleResult<()> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        load_word(&self.memory, pc, result, b, c)
    }

    pub(crate) fn store_byte(&mut self, a: Word, b: Word, c: Word) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        store_byte(&mut self.memory, owner, self.registers.pc_mut(), a, b, c)
    }

    pub(crate) fn store_word(&mut self, a: Word, b: Word, c: Imm12) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        store_word(&mut self.memory, owner, self.registers.pc_mut(), a, b, c)
    }

    /// Expand heap by `a` bytes.
    pub fn allocate(&mut self, a: Word) -> SimpleResult<()> {
        let (SystemRegisters { hp, sp, .. }, _) = split_registers(&mut self.registers);
        try_allocate(hp, sp.as_ref(), a, &mut self.memory)
    }

    pub(crate) fn malloc(&mut self, a: Word) -> SimpleResult<()> {
        let (SystemRegisters { hp, sp, pc, .. }, _) =
            split_registers(&mut self.registers);
        malloc(hp, sp.as_ref(), pc, a, &mut self.memory)
    }

    pub(crate) fn memclear(&mut self, a: Word, b: Word) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        memclear(&mut self.memory, owner, self.registers.pc_mut(), a, b)
    }

    pub(crate) fn memcopy(&mut self, a: Word, b: Word, c: Word) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        memcopy(&mut self.memory, owner, self.registers.pc_mut(), a, b, c)
    }

    pub(crate) fn memeq(
        &mut self,
        ra: RegisterId,
        b: Word,
        c: Word,
        d: Word,
    ) -> SimpleResult<()> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        memeq(&mut self.memory, result, pc, b, c, d)
    }
}

/// Update stack pointer, checking for validity first.
pub(crate) fn try_update_stack_pointer(
    mut sp: RegMut<SP>,
    ssp: Reg<SSP>,
    hp: Reg<HP>,
    new_sp: Word,
    memory: &mut Memory,
) -> SimpleResult<()> {
    if new_sp < *ssp {
        Err(PanicReason::MemoryOverflow.into())
    } else if new_sp > *hp {
        Err(PanicReason::MemoryGrowthOverlap.into())
    } else {
        *sp = new_sp;
        memory.grow_stack(new_sp)?;
        Ok(())
    }
}

pub(crate) fn stack_pointer_overflow<F>(
    sp: RegMut<SP>,
    ssp: Reg<SSP>,
    hp: Reg<HP>,
    pc: RegMut<PC>,
    f: F,
    v: Word,
    memory: &mut Memory,
) -> SimpleResult<()>
where
    F: FnOnce(Word, Word) -> (Word, bool),
{
    let (new_sp, overflow) = f(*sp, v);

    if overflow {
        return Err(PanicReason::MemoryOverflow.into())
    }

    try_update_stack_pointer(sp, ssp, hp, new_sp, memory)?;
    Ok(inc_pc(pc)?)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn push_selected_registers(
    memory: &mut Memory,
    sp: RegMut<SP>,
    ssp: Reg<SSP>,
    hp: Reg<HP>,
    pc: RegMut<PC>,
    program_regs: &ProgramRegisters,
    segment: ProgramRegistersSegment,
    bitmask: Imm24,
) -> SimpleResult<()> {
    let bitmask = bitmask.to_u32();

    // First update the new stack pointer, as that's the only error condition
    let count: u64 = bitmask.count_ones().into();
    let write_size = count
        .checked_mul(WORD_SIZE as u64)
        .expect("Bitmask size times 8 can never oveflow");
    let write_at = *sp;
    // If this would overflow, the stack pointer update below will fail
    let new_sp = write_at.saturating_add(write_size);
    try_update_stack_pointer(sp, ssp, hp, new_sp, memory)?;

    // Write the registers to the stack
    let mut it = memory
        .write_noownerchecks(write_at, write_size)?
        .chunks_exact_mut(WORD_SIZE);
    for (i, reg) in program_regs.segment(segment).iter().enumerate() {
        if (bitmask & (1 << i)) != 0 {
            let item = it
                .next()
                .expect("Memory range mismatched with register count");
            item.copy_from_slice(&reg.to_be_bytes());
        }
    }

    Ok(inc_pc(pc)?)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pop_selected_registers(
    memory: &mut Memory,
    sp: RegMut<SP>,
    ssp: Reg<SSP>,
    hp: Reg<HP>,
    pc: RegMut<PC>,
    program_regs: &mut ProgramRegisters,
    segment: ProgramRegistersSegment,
    bitmask: Imm24,
) -> SimpleResult<()> {
    let bitmask = bitmask.to_u32();

    // First update the stack pointer, as that's the only error condition
    let count: u64 = bitmask.count_ones().into();
    let size_in_stack = count
        .checked_mul(WORD_SIZE as u64)
        .expect("Bitmask size times 8 can never oveflow");
    let new_sp = sp
        .checked_sub(size_in_stack)
        .ok_or(PanicReason::MemoryOverflow)?;
    try_update_stack_pointer(sp, ssp, hp, new_sp, memory)?;

    // Restore registers from the stack
    let mut it = memory.read(new_sp, size_in_stack)?.chunks_exact(WORD_SIZE);
    for (i, reg) in program_regs.segment_mut(segment).iter_mut().enumerate() {
        if (bitmask & (1 << i)) != 0 {
            let mut buf = [0u8; WORD_SIZE];
            buf.copy_from_slice(it.next().expect("Count mismatch"));
            *reg = Word::from_be_bytes(buf);
        }
    }

    Ok(inc_pc(pc)?)
}

pub(crate) fn load_byte(
    memory: &Memory,
    pc: RegMut<PC>,
    result: &mut Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    let [b] = memory.read_bytes(b.saturating_add(c))?;
    *result = b as Word;
    Ok(inc_pc(pc)?)
}

pub(crate) fn load_word(
    memory: &Memory,
    pc: RegMut<PC>,
    result: &mut Word,
    b: Word,
    c: Imm12,
) -> SimpleResult<()> {
    let offset = u64::from(c)
        .checked_mul(WORD_SIZE as u64)
        .expect("u12 * 8 cannot overflow a Word");
    let addr = b.checked_add(offset).ok_or(PanicReason::MemoryOverflow)?;
    *result = Word::from_be_bytes(memory.read_bytes(addr)?);
    Ok(inc_pc(pc)?)
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn store_byte(
    memory: &mut Memory,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    memory.write_bytes(owner, a.saturating_add(c), [b as u8])?;
    Ok(inc_pc(pc)?)
}

pub(crate) fn store_word(
    memory: &mut Memory,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Imm12,
) -> SimpleResult<()> {
    #[allow(clippy::arithmetic_side_effects)]
    let offset = u64::from(c)
        .checked_mul(WORD_SIZE as u64)
        .expect("12-bits number multiplied by 8 cannot overflow a Word");
    let addr = a.saturating_add(offset);
    memory.write_bytes(owner, addr, b.to_be_bytes())?;
    Ok(inc_pc(pc)?)
}

pub(crate) fn try_allocate(
    mut hp: RegMut<HP>,
    sp: Reg<SP>,
    a: Word,
    memory: &mut Memory,
) -> SimpleResult<()> {
    let (result, overflow) = hp.overflowing_sub(a);

    if overflow {
        return Err(PanicReason::MemoryOverflow.into());
    }

    memory.grow_heap(sp, result)?;
    *hp = result;
    Ok(())
}

pub(crate) fn malloc(
    hp: RegMut<HP>,
    sp: Reg<SP>,
    pc: RegMut<PC>,
    a: Word,
    memory: &mut Memory,
) -> SimpleResult<()> {
    try_allocate(hp, sp, a, memory)?;
    Ok(inc_pc(pc)?)
}

pub(crate) fn memclear(
    memory: &mut Memory,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
) -> SimpleResult<()> {
    memory.write(owner, a, b)?.fill(0);
    Ok(inc_pc(pc)?)
}

pub(crate) fn memcopy(
    memory: &mut Memory,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    let dst_range = memory.verify(a, c)?;
    let src_range = memory.verify(b, c)?;

    owner.verify_ownership(&dst_range.words())?;

    if dst_range.start() <= src_range.start() && src_range.start() < dst_range.end()
        || src_range.start() <= dst_range.start() && dst_range.start() < src_range.end()
        || dst_range.start() < src_range.end() && src_range.end() <= dst_range.end()
        || src_range.start() < dst_range.end() && dst_range.end() <= src_range.end()
    {
        return Err(PanicReason::MemoryWriteOverlap.into())
    }

    memory.memcopy_noownerchecks(a, b, c)?;

    Ok(inc_pc(pc)?)
}

pub(crate) fn memeq(
    memory: &mut Memory,
    result: &mut Word,
    pc: RegMut<PC>,
    b: Word,
    c: Word,
    d: Word,
) -> SimpleResult<()> {
    *result = (memory.read(b, d)? == memory.read(c, d)?) as Word;
    Ok(inc_pc(pc)?)
}

#[derive(Debug)]
pub struct OwnershipRegisters {
    pub(crate) sp: u64,
    pub(crate) ssp: u64,
    pub(crate) hp: u64,
    pub(crate) prev_hp: u64,
    pub(crate) context: Context,
}

impl OwnershipRegisters {
    pub(crate) fn new<S, Tx, Ecal>(vm: &Interpreter<S, Tx, Ecal>) -> Self {
        OwnershipRegisters {
            sp: vm.registers[RegId::SP],
            ssp: vm.registers[RegId::SSP],
            hp: vm.registers[RegId::HP],
            prev_hp: vm
                .frames
                .last()
                .map(|frame| frame.registers()[RegId::HP])
                .unwrap_or(0),
            context: vm.context.clone(),
        }
    }

    pub(crate) fn verify_ownership(
        &self,
        range: &Range<Word>,
    ) -> Result<(), PanicReason> {
        if self.has_ownership_range(range) {
            Ok(())
        } else {
            Err(PanicReason::MemoryOwnership)
        }
    }

    pub(crate) fn verify_internal_context(&self) -> Result<(), PanicReason> {
        if self.context.is_internal() {
            Ok(())
        } else {
            Err(PanicReason::ExpectedInternalContext)
        }
    }

    pub(crate) fn has_ownership_range(&self, range: &Range<Word>) -> bool {
        self.has_ownership_stack(range) || self.has_ownership_heap(range)
    }

    /// Empty range is owned iff the range.start is owned
    pub(crate) fn has_ownership_stack(&self, range: &Range<Word>) -> bool {
        if range.is_empty() && range.start == self.ssp {
            return true
        }

        if !(self.ssp..self.sp).contains(&range.start) {
            return false
        }

        if range.end > VM_MAX_RAM {
            return false
        }

        (self.ssp..=self.sp).contains(&range.end)
    }

    /// Empty range is owned iff the range.start is owned
    pub(crate) fn has_ownership_heap(&self, range: &Range<Word>) -> bool {
        // TODO implement fp->hp and (addr, size) validations
        // fp->hp
        // it means $hp from the previous context, i.e. what's saved in the
        // "Saved registers from previous context" of the call frame at
        // $fp`
        if range.start < self.hp {
            return false
        }

        let heap_end = if self.context.is_external() {
            VM_MAX_RAM
        } else {
            self.prev_hp
        };

        self.hp != heap_end && range.end <= heap_end
    }
}

/// Attempt copy from slice to memory, filling zero bytes when exceeding slice boundaries.
/// Performs overflow and memory range checks, but no ownership checks.
pub(crate) fn copy_from_slice_zero_fill_noownerchecks<A: ToAddr, B: ToAddr>(
    memory: &mut Memory,
    src: &[u8],
    dst_addr: A,
    src_offset: usize,
    len: B,
) -> SimpleResult<()> {
    let range = memory.verify(dst_addr, len)?;

    let src_end = src_offset.saturating_add(range.len()).min(src.len());
    let data = src.get(src_offset..src_end).unwrap_or_default();
    let (r_data, r_zero) = range.split_at_offset(data.len());

    memory
        .write_noownerchecks(r_data.start(), r_data.len())
        .expect("Range verified above")
        .copy_from_slice(data);
    memory
        .write_noownerchecks(r_zero.start(), r_zero.len())
        .expect("Range verified above")
        .fill(0);

    Ok(())
}

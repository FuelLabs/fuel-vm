#![cfg(feature = "alloc")]

use super::{
    internal::inc_pc,
    Interpreter,
};
use crate::{
    constraints::reg_key::*,
    consts::*,
    error::SimpleResult,
};

use fuel_asm::{
    Imm12,
    Imm24,
    PanicReason,
    RegId,
    RegW,
};
use fuel_types::{
    fmt_truncated_hex,
    Word,
};

use core::{
    fmt,
    ops::Range,
};

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

/// The trait for the memory.
pub trait Memory: AsRef<MemoryInstance> + AsMut<MemoryInstance> {}

impl<M> Memory for M where M: AsRef<MemoryInstance> + AsMut<MemoryInstance> {}

/// The memory of the VM, represented as stack and heap.
#[derive(Clone, Eq)]
pub struct MemoryInstance {
    /// Stack. Grows upwards.
    stack: Vec<u8>,
    /// Heap. Grows downwards from MEM_SIZE.
    heap: Vec<u8>,
    /// Lowest allowed heap address, i.e. hp register value.
    /// This is needed since we can allocate extra heap for performance reasons.
    hp: usize,
}

impl Default for MemoryInstance {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for MemoryInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Memory {{ stack: ")?;
        fmt_truncated_hex::<16>(&self.stack, f)?;
        write!(f, ", heap: ")?;
        let off = self.hp.saturating_sub(self.heap_offset());
        fmt_truncated_hex::<16>(&self.heap[off..], f)?;
        write!(f, ", hp: {} }}", self.hp)
    }
}

impl PartialEq for MemoryInstance {
    /// Equality comparison of the accessible memory.
    #[allow(clippy::arithmetic_side_effects)] // Safety: hp is kept valid everywhere
    fn eq(&self, other: &Self) -> bool {
        self.stack == other.stack && self.hp == other.hp && {
            let self_hs = self.hp - self.heap_offset();
            let other_hs = other.hp - other.heap_offset();
            self.heap[self_hs..] == other.heap[other_hs..]
        }
    }
}

impl AsRef<MemoryInstance> for MemoryInstance {
    fn as_ref(&self) -> &MemoryInstance {
        self
    }
}
impl AsMut<MemoryInstance> for MemoryInstance {
    fn as_mut(&mut self) -> &mut MemoryInstance {
        self
    }
}

impl MemoryInstance {
    /// Create a new VM memory.
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            heap: Vec::new(),
            hp: MEM_SIZE,
        }
    }

    /// Resets memory to initial state, keeping the original allocations.
    pub fn reset(&mut self) {
        self.stack.truncate(0);
        self.hp = MEM_SIZE;
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
        if new_sp > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow);
        }
        #[allow(clippy::cast_possible_truncation)] // Safety: VM_MAX_RAM is usize
        let new_sp = new_sp as usize;

        if new_sp > self.stack.len() {
            if new_sp > self.hp {
                return Err(PanicReason::MemoryGrowthOverlap)
            }

            self.stack.resize(new_sp, 0);
        }
        Ok(())
    }

    /// Grows the heap by `amount` bytes. Updates hp register.
    pub fn grow_heap_by(
        &mut self,
        sp_reg: Reg<SP>,
        mut hp_reg: RegMut<HP>,
        amount: Word,
    ) -> Result<(), PanicReason> {
        debug_assert_eq!(
            self.hp as Word, *hp_reg,
            "HP register changed without memory update"
        );

        let amount = usize::try_from(amount).map_err(|_| PanicReason::MemoryOverflow)?;
        let new_hp = self
            .hp
            .checked_sub(amount)
            .ok_or(PanicReason::MemoryOverflow)?;

        if (new_hp as Word) < *sp_reg {
            return Err(PanicReason::MemoryGrowthOverlap)
        }

        #[allow(clippy::arithmetic_side_effects)] // Safety: self.hp is in heap
        let new_len = MEM_SIZE - new_hp;

        #[allow(clippy::arithmetic_side_effects)] // Safety: self.hp is in heap
        if self.heap.len() >= new_len {
            // No need to reallocate, but we need to zero the new space
            // in case it was used before a memory reset.
            let start = new_hp - self.heap_offset();
            let end = self.hp - self.heap_offset();
            self.heap[start..end].fill(0);
        } else {
            // Reallocation is needed.
            // To reduce frequent reallocations, allocate at least 256 bytes at once.
            // After that, double the allocation every time.
            let cap = new_len.next_power_of_two().clamp(256, MEM_SIZE);
            let old_len = self.heap.len();
            let prefix_zeroes = cap - old_len;
            self.heap.resize(cap, 0);
            self.heap.copy_within(..old_len, prefix_zeroes);
            self.heap[..prefix_zeroes].fill(0);
        }

        self.hp = new_hp;
        *hp_reg = new_hp as Word;

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
        owner.verify_ownership(&range)?;
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
impl From<Vec<u8>> for MemoryInstance {
    fn from(stack: Vec<u8>) -> Self {
        Self {
            stack,
            ..Self::new()
        }
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl Index<Range<usize>> for MemoryInstance {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        self.read(index.start, index.len())
            .expect("Memory range out of bounds")
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl Index<RangeFrom<usize>> for MemoryInstance {
    type Output = [u8];

    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self[index.start..MEM_SIZE]
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl Index<RangeTo<usize>> for MemoryInstance {
    type Output = [u8];

    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self[0..index.end]
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl IndexMut<Range<usize>> for MemoryInstance {
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

impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal>
where
    M: Memory,
{
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
        stack_pointer_overflow(
            sp,
            ssp.as_ref(),
            hp.as_ref(),
            pc,
            f,
            v,
            self.memory.as_mut(),
        )
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
            self.memory.as_mut(),
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
            self.memory.as_mut(),
            sp,
            ssp.as_ref(),
            hp.as_ref(),
            pc,
            &mut program_regs,
            segment,
            bitmask,
        )
    }

    pub(crate) fn load_byte(&mut self, ra: RegW, b: Word, c: Word) -> SimpleResult<()> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        load_byte(self.memory.as_ref(), pc, &mut w[ra], b, c)
    }

    pub(crate) fn load_word(&mut self, ra: RegW, b: Word, c: Imm12) -> SimpleResult<()> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        load_word(self.memory.as_ref(), pc, &mut w[ra], b, c)
    }

    pub(crate) fn store_byte(&mut self, a: Word, b: Word, c: Word) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        store_byte(
            self.memory.as_mut(),
            owner,
            self.registers.pc_mut(),
            a,
            b,
            c,
        )
    }

    pub(crate) fn store_word(&mut self, a: Word, b: Word, c: Imm12) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        store_word(
            self.memory.as_mut(),
            owner,
            self.registers.pc_mut(),
            a,
            b,
            c,
        )
    }

    /// Expand heap by `amount` bytes.
    pub fn allocate(&mut self, amount: Word) -> SimpleResult<()> {
        let (SystemRegisters { hp, sp, .. }, _) = split_registers(&mut self.registers);
        self.memory.as_mut().grow_heap_by(sp.as_ref(), hp, amount)?;
        Ok(())
    }

    pub(crate) fn malloc(&mut self, a: Word) -> SimpleResult<()> {
        let (SystemRegisters { hp, sp, pc, .. }, _) =
            split_registers(&mut self.registers);
        malloc(hp, sp.as_ref(), pc, a, self.memory.as_mut())
    }

    pub(crate) fn memclear(&mut self, a: Word, b: Word) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        memclear(self.memory.as_mut(), owner, self.registers.pc_mut(), a, b)
    }

    pub(crate) fn memcopy(&mut self, a: Word, b: Word, c: Word) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        memcopy(
            self.memory.as_mut(),
            owner,
            self.registers.pc_mut(),
            a,
            b,
            c,
        )
    }

    pub(crate) fn memeq(
        &mut self,
        ra: RegW,
        b: Word,
        c: Word,
        d: Word,
    ) -> SimpleResult<()> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        memeq(self.memory.as_mut(), &mut w[ra], pc, b, c, d)
    }
}

/// Update stack pointer, checking for validity first.
pub(crate) fn try_update_stack_pointer(
    mut sp: RegMut<SP>,
    ssp: Reg<SSP>,
    hp: Reg<HP>,
    new_sp: Word,
    memory: &mut MemoryInstance,
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
    memory: &mut MemoryInstance,
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
    memory: &mut MemoryInstance,
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
    memory: &mut MemoryInstance,
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
    memory: &MemoryInstance,
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
    memory: &MemoryInstance,
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
    memory: &mut MemoryInstance,
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
    memory: &mut MemoryInstance,
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

pub(crate) fn malloc(
    hp: RegMut<HP>,
    sp: Reg<SP>,
    pc: RegMut<PC>,
    amount: Word,
    memory: &mut MemoryInstance,
) -> SimpleResult<()> {
    memory.grow_heap_by(sp, hp, amount)?;
    Ok(inc_pc(pc)?)
}

pub(crate) fn memclear(
    memory: &mut MemoryInstance,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
) -> SimpleResult<()> {
    memory.write(owner, a, b)?.fill(0);
    Ok(inc_pc(pc)?)
}

pub(crate) fn memcopy(
    memory: &mut MemoryInstance,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    let dst_range = memory.verify(a, c)?;
    let src_range = memory.verify(b, c)?;

    if dst_range.start() <= src_range.start() && src_range.start() < dst_range.end()
        || src_range.start() <= dst_range.start() && dst_range.start() < src_range.end()
        || dst_range.start() < src_range.end() && src_range.end() <= dst_range.end()
        || src_range.start() < dst_range.end() && dst_range.end() <= src_range.end()
    {
        return Err(PanicReason::MemoryWriteOverlap.into())
    }

    owner.verify_ownership(&dst_range)?;
    memory.memcopy_noownerchecks(a, b, c)?;

    Ok(inc_pc(pc)?)
}

pub(crate) fn memeq(
    memory: &mut MemoryInstance,
    result: &mut Word,
    pc: RegMut<PC>,
    b: Word,
    c: Word,
    d: Word,
) -> SimpleResult<()> {
    *result = (memory.read(b, d)? == memory.read(c, d)?) as Word;
    Ok(inc_pc(pc)?)
}

#[derive(Debug, Clone, Copy)]
pub struct OwnershipRegisters {
    pub(crate) sp: u64,
    pub(crate) ssp: u64,
    pub(crate) hp: u64,
    /// Previous heap pointer, used for external contexts.
    /// Otherwise, it's just memory size.
    pub(crate) prev_hp: u64,
}

impl OwnershipRegisters {
    pub(crate) fn new<M, S, Tx, Ecal>(vm: &Interpreter<M, S, Tx, Ecal>) -> Self {
        let prev_hp = vm
            .frames
            .last()
            .map(|frame| frame.registers()[RegId::HP])
            .unwrap_or(VM_MAX_RAM);

        OwnershipRegisters {
            sp: vm.registers[RegId::SP],
            ssp: vm.registers[RegId::SSP],
            hp: vm.registers[RegId::HP],
            prev_hp,
        }
    }

    /// Create an instance that only allows stack writes.
    pub(crate) fn only_allow_stack_write(sp: u64, ssp: u64, hp: u64) -> Self {
        debug_assert!(sp <= VM_MAX_RAM);
        debug_assert!(ssp <= VM_MAX_RAM);
        debug_assert!(hp <= VM_MAX_RAM);
        debug_assert!(ssp <= sp);
        debug_assert!(sp <= hp);
        OwnershipRegisters {
            sp,
            ssp,
            hp,
            prev_hp: hp,
        }
    }

    /// Allows all writes, whole memory is stack.allocated
    #[cfg(test)]
    pub(crate) fn test_full_stack() -> Self {
        OwnershipRegisters {
            sp: VM_MAX_RAM,
            ssp: 0,
            hp: VM_MAX_RAM,
            prev_hp: VM_MAX_RAM,
        }
    }

    pub(crate) fn verify_ownership(
        &self,
        range: &MemoryRange,
    ) -> Result<(), PanicReason> {
        if self.has_ownership_range(&range.words()) {
            Ok(())
        } else {
            Err(PanicReason::MemoryOwnership)
        }
    }

    pub fn has_ownership_range(&self, range: &Range<Word>) -> bool {
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
        if range.is_empty() && range.start == self.hp {
            return true
        }

        if range.start < self.hp {
            return false
        }

        self.hp != self.prev_hp && range.end <= self.prev_hp
    }
}

/// Attempt copy from slice to memory, filling zero bytes when exceeding slice boundaries.
/// Performs overflow and memory range checks, but no ownership checks.
/// Note that if `src_offset` is larger than `src.len()`, the whole range will be
/// zero-filled.
pub(crate) fn copy_from_slice_zero_fill<A: ToAddr, B: ToAddr>(
    memory: &mut MemoryInstance,
    owner: OwnershipRegisters,
    src: &[u8],
    dst_addr: A,
    src_offset: Word,
    len: B,
) -> SimpleResult<()> {
    let range = memory.write(owner, dst_addr, len)?;

    // Special-case the ranges that are completely out of bounds,
    // to avoid platform-dependenct usize conversion.
    if src_offset >= src.len() as Word {
        range[..].fill(0);
    } else {
        // Safety: since we check above that this is not larger than `src.len()`,
        // which is `usize`, the cast never truncates.
        #[allow(clippy::cast_possible_truncation)]
        let src_offset = src_offset as usize;
        let src_end = src_offset.saturating_add(range.len()).min(src.len());
        let data = src.get(src_offset..src_end).unwrap_or(&[]);

        range[..data.len()].copy_from_slice(data);
        range[data.len()..].fill(0);
    }
    Ok(())
}

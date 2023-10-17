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

use fuel_asm::{
    Imm24,
    PanicReason,
    RegId,
};
use fuel_types::{
    RegisterId,
    Word,
};

use alloc::boxed::Box;
use core::{
    ops,
    ops::Range,
};

pub type Memory<const SIZE: usize> = Box<[u8; SIZE]>;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod allocation_tests;

#[cfg(test)]
mod stack_tests;

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

/// Memory range representation for the VM, checked to be in-bounds on construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryRange(ops::Range<usize>);

impl Default for MemoryRange {
    fn default() -> Self {
        Self(0..0)
    }
}

impl ops::Deref for MemoryRange {
    type Target = ops::Range<usize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MemoryRange {
    /// Create a new memory range represented as `[address, address + size[`.
    pub fn new<A: ToAddr, B: ToAddr>(address: A, size: B) -> Result<Self, PanicReason> {
        let start = address.to_addr()?;
        let size = size.to_addr()?;
        let end = start.checked_add(size).ok_or(PanicReason::MemoryOverflow)?;

        if end > MEM_SIZE {
            return Err(PanicReason::MemoryOverflow)
        }

        Ok(Self(start..end))
    }

    /// Create a new const sized memory range.
    pub fn new_const<A: ToAddr, const SIZE: usize>(
        address: A,
    ) -> Result<Self, PanicReason> {
        Self::new(address, SIZE)
    }

    /// Uses a overflow-capturing operator to combine `base` and `offset`
    /// into the start of a memory range, returning an error on overflow,
    /// and then checks that the resulting range is within the VM memory.
    pub fn new_overflowing_op<A: ToAddr, T: ToAddr, F>(
        overflowing_op: F,
        base: T,
        offset: T,
        size: A,
    ) -> Result<Self, PanicReason>
    where
        F: FnOnce(T, T) -> (T, bool),
    {
        let (addr, overflow) = overflowing_op(base, offset);
        if overflow {
            return Err(PanicReason::MemoryOverflow)
        }
        Self::new(addr, size)
    }

    /// Truncate a memory range to a new size if it's smaller then current one.
    pub fn truncated(&self, limit: usize) -> Self {
        Self::new(self.start, self.len().min(limit))
            .expect("Cannot overflow on truncation")
    }

    /// Return `true` if the length is `0`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Convert to a raw `usize` range.
    pub fn usizes(&self) -> ops::Range<usize> {
        self.start..self.end
    }

    /// Convert to a raw `Word` range.
    pub fn words(&self) -> ops::Range<Word> {
        self.start as Word..self.end as Word
    }

    /// Return an owned memory slice with a relative address to the heap space
    /// defined in `r[$hp]`. Panics if the range is not within the heap space.
    #[cfg(test)]
    pub fn to_heap<S, Tx>(self, vm: &Interpreter<S, Tx>) -> Self {
        let hp = vm.registers()[RegId::HP] as usize;
        let start = self.start.checked_add(hp).expect("Overflow");
        let end = self.end.checked_add(hp).expect("Overflow");
        if end > MEM_SIZE {
            panic!("Invalid heap range");
        }
        Self(start..end)
    }

    /// Splits range at given relative offset. Panics if offset > range length.
    pub fn split_at_offset(self, at: usize) -> (Self, Self) {
        let mid = self.0.start + at;
        assert!(mid <= self.0.end);
        (Self(self.0.start..mid), Self(mid..self.0.end))
    }

    /// This function is safe because it is only used to shrink the range
    /// and worst case the range will be empty.
    pub fn shrink_end(&mut self, by: usize) {
        self.0 = self.0.start..self.0.end.saturating_sub(by);
    }

    /// This function is safe because it is only used to grow the range
    /// and worst case the range will be empty.
    pub fn grow_start(&mut self, by: usize) {
        self.0 = self.0.start.saturating_add(by)..self.0.end;
    }

    /// Get the memory slice for this range.
    pub fn read(self, memory: &[u8; MEM_SIZE]) -> &[u8] {
        &memory[self.0]
    }

    /// Get the mutable memory slice for this range.
    pub fn write(self, memory: &mut [u8; MEM_SIZE]) -> &mut [u8] {
        &mut memory[self.0]
    }
}

impl<S, Tx> Interpreter<S, Tx> {
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
        stack_pointer_overflow(sp, ssp.as_ref(), hp.as_ref(), pc, f, v)
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
            &self.memory,
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
        c: Word,
    ) -> SimpleResult<()> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        load_word(&self.memory, pc, result, b, c)
    }

    pub(crate) fn store_byte(&mut self, a: Word, b: Word, c: Word) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        store_byte(&mut self.memory, owner, self.registers.pc_mut(), a, b, c)
    }

    pub(crate) fn store_word(&mut self, a: Word, b: Word, c: Word) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        store_word(&mut self.memory, owner, self.registers.pc_mut(), a, b, c)
    }

    pub(crate) fn malloc(&mut self, a: Word) -> SimpleResult<()> {
        let (SystemRegisters { hp, sp, pc, .. }, _) =
            split_registers(&mut self.registers);
        malloc(hp, sp.as_ref(), pc, a)
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
) -> SimpleResult<()> {
    if new_sp >= *hp || new_sp < *ssp {
        Err(PanicReason::MemoryOverflow.into())
    } else {
        *sp = new_sp;
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
) -> SimpleResult<()>
where
    F: FnOnce(Word, Word) -> (Word, bool),
{
    let (new_sp, overflow) = f(*sp, v);

    if overflow {
        return Err(PanicReason::MemoryOverflow.into())
    }

    try_update_stack_pointer(sp, ssp, hp, new_sp)?;
    Ok(inc_pc(pc)?)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn push_selected_registers(
    memory: &mut [u8; MEM_SIZE],
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
    let stack_range = MemoryRange::new(*sp, count * 8)?;
    try_update_stack_pointer(sp, ssp, hp, stack_range.words().end)?;

    // Write the registers to the stack
    let mut it = memory[stack_range.usizes()].chunks_exact_mut(8);
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
    memory: &[u8; MEM_SIZE],
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
    let size_in_stack = count * 8;
    let new_sp = sp
        .checked_sub(size_in_stack)
        .ok_or(PanicReason::MemoryOverflow)?;
    try_update_stack_pointer(sp, ssp, hp, new_sp)?;
    let stack_range = MemoryRange::new(new_sp, size_in_stack)?.usizes();

    // Restore registers from the stack
    let mut it = memory[stack_range].chunks_exact(8);
    for (i, reg) in program_regs.segment_mut(segment).iter_mut().enumerate() {
        if (bitmask & (1 << i)) != 0 {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(it.next().expect("Count mismatch"));
            *reg = Word::from_be_bytes(buf);
        }
    }

    Ok(inc_pc(pc)?)
}

pub(crate) fn load_byte(
    memory: &[u8; MEM_SIZE],
    pc: RegMut<PC>,
    result: &mut Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    let range = MemoryRange::new_overflowing_op(Word::overflowing_add, b, c, 1u64)?;
    *result = memory[range.start] as Word;
    Ok(inc_pc(pc)?)
}

pub(crate) fn load_word(
    memory: &[u8; MEM_SIZE],
    pc: RegMut<PC>,
    result: &mut Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    // C is expressed in words; mul by 8. This cannot overflow since it's a 12 bit
    // immediate value.
    let addr = b.checked_add(c * 8).ok_or(PanicReason::MemoryOverflow)?;
    *result = Word::from_be_bytes(read_bytes(memory, addr)?);
    Ok(inc_pc(pc)?)
}

pub(crate) fn store_byte(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    write_bytes(memory, owner, a.saturating_add(c), [b as u8])?;
    Ok(inc_pc(pc)?)
}

pub(crate) fn store_word(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    // C is expressed in words; mul by 8. This cannot overflow since it's a 12 bit
    // immediate value.
    let addr = a.checked_add(c * 8).ok_or(PanicReason::MemoryOverflow)?;
    write_bytes(memory, owner, addr, b.to_be_bytes())?;
    Ok(inc_pc(pc)?)
}

pub(crate) fn malloc(
    mut hp: RegMut<HP>,
    sp: Reg<SP>,
    pc: RegMut<PC>,
    a: Word,
) -> SimpleResult<()> {
    let (result, overflow) = hp.overflowing_sub(a);

    if overflow || result < *sp {
        Err(PanicReason::MemoryOverflow.into())
    } else {
        *hp = result;

        Ok(inc_pc(pc)?)
    }
}

pub(crate) fn memclear(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
) -> SimpleResult<()> {
    let range = MemoryRange::new(a, b)?;
    owner.verify_ownership(&range)?;
    memory[range.usizes()].fill(0);
    Ok(inc_pc(pc)?)
}

pub(crate) fn memcopy(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    let dst_range = MemoryRange::new(a, c)?;
    let src_range = MemoryRange::new(b, c)?;

    owner.verify_ownership(&dst_range)?;

    if dst_range.start <= src_range.start && src_range.start < dst_range.end
        || src_range.start <= dst_range.start && dst_range.start < src_range.end
        || dst_range.start < src_range.end && src_range.end <= dst_range.end
        || src_range.start < dst_range.end && dst_range.end <= src_range.end
    {
        return Err(PanicReason::MemoryWriteOverlap.into())
    }

    let len = src_range.len();
    if a <= b {
        let (dst, src) = memory.split_at_mut(src_range.start);
        dst[dst_range.usizes()].copy_from_slice(&src[..len]);
    } else {
        let (src, dst) = memory.split_at_mut(dst_range.start);
        dst[..len].copy_from_slice(&src[src_range.usizes()]);
    }

    Ok(inc_pc(pc)?)
}

pub(crate) fn memeq(
    memory: &mut [u8; MEM_SIZE],
    result: &mut Word,
    pc: RegMut<PC>,
    b: Word,
    c: Word,
    d: Word,
) -> SimpleResult<()> {
    let range1 = MemoryRange::new(b, d)?;
    let range2 = MemoryRange::new(c, d)?;
    *result = (memory[range1.usizes()] == memory[range2.usizes()]) as Word;
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
    pub(crate) fn new<S, Tx>(vm: &Interpreter<S, Tx>) -> Self {
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
        range: &MemoryRange,
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

    pub(crate) fn has_ownership_range(&self, range: &MemoryRange) -> bool {
        let range = range.words();
        self.has_ownership_stack(&range) || self.has_ownership_heap(&range)
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

pub(crate) fn try_mem_write<A: ToAddr>(
    addr: A,
    data: &[u8],
    owner: OwnershipRegisters,
    memory: &mut [u8; MEM_SIZE],
) -> SimpleResult<()> {
    let range = MemoryRange::new(addr, data.len())?;
    owner.verify_ownership(&range)?;
    memory[range.usizes()].copy_from_slice(data);
    Ok(())
}

pub(crate) fn try_zeroize<A: ToAddr, B: ToAddr>(
    addr: A,
    len: B,
    owner: OwnershipRegisters,
    memory: &mut [u8; MEM_SIZE],
) -> SimpleResult<()> {
    let range = MemoryRange::new(addr, len)?;
    owner.verify_ownership(&range)?;
    memory[range.usizes()].fill(0);
    Ok(())
}

/// Reads a constant-sized byte array from memory, performing overflow and memory range
/// checks.
pub(crate) fn read_bytes<const COUNT: usize>(
    memory: &[u8; MEM_SIZE],
    addr: Word,
) -> Result<[u8; COUNT], PanicReason> {
    let range = MemoryRange::new_const::<_, COUNT>(addr)?;
    Ok(<[u8; COUNT]>::try_from(&memory[range.usizes()])
        .unwrap_or_else(|_| unreachable!()))
}

/// Writes a constant-sized byte array to memory, performing overflow, memory range and
/// ownership checks.
pub(crate) fn write_bytes<const COUNT: usize>(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    addr: Word,
    bytes: [u8; COUNT],
) -> SimpleResult<()> {
    let range = MemoryRange::new_const::<_, COUNT>(addr)?;
    owner.verify_ownership(&range)?;
    memory[range.usizes()].copy_from_slice(&bytes);
    Ok(())
}

/// Attempt copy from slice to memory, filling zero bytes when exceeding slice boundaries.
/// Performs overflow and memory range checks, but no ownership checks.
pub(crate) fn copy_from_slice_zero_fill_noownerchecks<A: ToAddr, B: ToAddr>(
    memory: &mut [u8; MEM_SIZE],
    src: &[u8],
    dst_addr: A,
    src_offset: usize,
    len: B,
) -> SimpleResult<()> {
    let range = MemoryRange::new(dst_addr, len)?;

    let src_end = src_offset.saturating_add(range.len()).min(src.len());
    let data = src.get(src_offset..src_end).unwrap_or_default();
    let (r_data, r_zero) = range.split_at_offset(data.len());
    memory[r_data.usizes()].copy_from_slice(data);
    memory[r_zero.usizes()].fill(0);

    Ok(())
}

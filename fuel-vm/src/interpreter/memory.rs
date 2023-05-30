// #[cfg(test)]
// mod allocation_tests; // TODO
mod operations;
mod ownership;

#[cfg(test)]
mod tests;

pub use self::ownership::OwnershipRegisters;

use std::{io, iter, ops::Range};

use derivative::Derivative;
use fuel_asm::PanicReason;
use fuel_types::{fmt_truncated_hex, Word};

use crate::{consts::*, prelude::RuntimeError};

/// Page size, in bytes.
pub const VM_PAGE_SIZE: usize = 16 * (1 << 10); // 16 KiB

/// A single page of memory.
pub type MemoryPage = [u8; VM_PAGE_SIZE];

/// A zeroed page of memory.
pub const ZERO_PAGE: MemoryPage = [0u8; VM_PAGE_SIZE];

static_assertions::const_assert!(VM_PAGE_SIZE < MEM_SIZE);
static_assertions::const_assert!(MEM_SIZE % VM_PAGE_SIZE == 0);

/// A range of memory, checked to be within the VM memory bounds upon construction.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryRange(Range<usize>);

impl MemoryRange {
    /// Returns `None` if the range doesn't fall within the VM memory.
    pub fn try_new(start: Word, len: Word) -> Result<Self, PanicReason> {
        let start: usize = start.try_into().map_err(|_| PanicReason::MemoryOverflow)?;
        let len: usize = len.try_into().map_err(|_| PanicReason::MemoryOverflow)?;
        Self::try_new_usize(start, len)
    }

    /// Returns `None` if the range doesn't fall within the VM memory.
    pub fn try_new_usize(start: usize, len: usize) -> Result<Self, PanicReason> {
        let end = start.checked_add(len).ok_or(PanicReason::MemoryOverflow)?;

        if end > MEM_SIZE {
            return Err(PanicReason::MemoryOverflow);
        }

        Ok(Self(start..end))
    }

    /// Converts this to a `usize` range. This is needed because `Range` doesn't implement `Copy`.
    pub fn as_usizes(&self) -> Range<usize> {
        self.0.clone()
    }

    /// Converts this to a `Word` range
    pub fn as_words(&self) -> Range<Word> {
        (self.start as Word)..(self.end as Word)
    }

    /// Checks if a range falls fully within another range
    pub fn contains_range(&self, inner: &Self) -> bool {
        self.contains(&inner.start) && inner.end <= self.end
    }

    /// Computes the overlap (intersection) of two ranges. Returns `None` if the ranges do not overlap.
    pub fn overlap_with(&self, other: &Self) -> Option<Self> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start < end {
            Some(Self(start..end))
        } else {
            None
        }
    }

    /// Checks that a range is fully contained within another range, and then returns
    /// the self as offset relative to the outer range.
    pub fn relative_to(&self, outer: &Self) -> Option<Self> {
        if outer.contains_range(self) {
            Some(Self(self.start - outer.start..self.end - outer.start))
        } else {
            None
        }
    }

    /// Shrink the into the given subrange.
    /// Returns `None` if the subrange is not fully contained within the range.
    pub fn subrange(&self, offset: usize, len: usize) -> Option<Self> {
        let new_start = self.start.checked_add(offset)?;
        let new_end = new_start.checked_add(len)?;

        if new_end > self.end {
            return None;
        }

        Some(Self(new_start..new_end))
    }

    /// Splits the range into two ranges at the given offset.
    /// Returns `None` if the offset is not within the range.
    pub fn split_at(&self, offset: usize) -> Option<(Self, Self)> {
        if offset > self.end {
            return None;
        }

        Some((Self(self.start..offset), Self(offset..self.end)))
    }
}

impl std::ops::Deref for MemoryRange {
    type Target = Range<usize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Number of new pages allocated by a memory allocation request.
#[must_use = "Gas charging is required when new pages are allacted"]
pub struct AllocatedPages(pub usize);

/// Stack and heap memory regions would overlap.
#[derive(Debug)]
#[must_use = "Gas charging is required when new pages are allacted"]
pub struct StackAndHeapOverlap;

/// The memory of a single VM instance.
/// Note that even though the memory is divided into stack and heap pages,
/// those names are only descriptive and do not imply any special behavior.
/// When doing reads, both stack and heap pages are treated the same.
#[derive(Clone, Derivative, Eq)]
#[derivative(Debug)]
pub struct VmMemory {
    /// Stack memory is allocated in from the beginning of the address space.
    #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))]
    stack: Vec<u8>,
    /// Heap memory is allocated in from the end of the address space, in reverse order.
    #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))]
    heap: Vec<u8>,
}
impl VmMemory {
    /// Create a new empty VM memory instance.
    pub const fn new() -> Self {
        Self {
            stack: Vec::new(),
            heap: Vec::new(),
        }
    }

    /// Allocates full memory range for stack.
    #[cfg(test)]
    pub fn fully_allocated() -> Self {
        let mut mem = Self::new();
        let _ = mem.update_allocations(VM_MAX_RAM, VM_MAX_RAM).unwrap();
        mem
    }

    fn unallocated(&self) -> usize {
        MEM_SIZE
            .checked_sub(self.stack.len() + self.heap.len())
            .expect("Memory over allocated")
    }

    /// Given the stack and heap pointers, ensures that the stack is large enough to contain it.
    /// Also ensures that stack and heap regions cannot overlap.
    /// This must be called every time new stack or heap space is allocated.
    pub fn update_allocations(&mut self, sp: Word, hp: Word) -> Result<AllocatedPages, StackAndHeapOverlap> {
        if hp < sp {
            return Err(StackAndHeapOverlap);
        }

        // To guard against the last page being allocated twice
        let available_pages = self.unallocated() / VM_PAGE_SIZE;
        let mut new_pages = 0;

        while self.stack.len() < sp as usize && new_pages < available_pages {
            self.stack.extend(&ZERO_PAGE);
            new_pages += 1;
        }

        while self.heap_range().start > hp as usize && new_pages < available_pages {
            self.heap.extend(&ZERO_PAGE);
            new_pages += 1;
        }

        Ok(AllocatedPages(new_pages))
    }

    fn stack_range(&self) -> MemoryRange {
        MemoryRange(0..self.stack.len())
    }

    fn heap_range(&self) -> MemoryRange {
        // Never wraps, heap isn't larger than the address space
        let heap_start = MEM_SIZE - self.heap.len();
        MemoryRange(heap_start..MEM_SIZE)
    }

    /// Read-only iteration of the full memory space, including unallocated pages filled with zeroes.
    pub fn iter(&self) -> impl Iterator<Item = &u8> + '_ {
        self.stack
            .iter()
            .chain(iter::repeat(&0u8).take(self.unallocated()))
            .chain(self.heap.iter())
    }

    /// Read given number of bytes of memory at address
    pub fn read(&self, addr: usize, count: usize) -> Result<impl Iterator<Item = &u8> + '_, RuntimeError> {
        let end = addr.saturating_add(count).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        Ok(self.iter().skip(addr).take(count))
    }

    /// Read a range of memory.
    pub fn read_range(&self, range: MemoryRange) -> Result<impl Iterator<Item = &u8> + '_, RuntimeError> {
        self.read(range.start, range.len())
    }

    /// Read from memory into anything that implements `std::io::Write`.
    pub fn read_into<W: io::Write>(&self, addr: usize, count: usize, mut target: W) -> Result<(), RuntimeError> {
        // TODO: optimize for chunks?
        self.read(addr, count)?.try_for_each(|b| target.write_all(&[*b]))?;
        Ok(())
    }

    /// Read a constant size byte array from the memory.
    /// This operation copies the data and is intended for small reads only.
    pub fn read_bytes<const S: usize>(&self, addr: usize) -> Result<[u8; S], RuntimeError> {
        let mut result = [0u8; S];

        for (dst, src) in result.iter_mut().zip(self.read(addr, S)?) {
            *dst = *src;
        }

        Ok(result)
    }

    /// Read a single byte of memory.
    pub fn at(&self, addr: usize) -> Result<u8, RuntimeError> {
        let b: [u8; 1] = self.read_bytes(addr)?;
        Ok(b[0])
    }

    /// Zero memory memory without performing ownership checks.
    pub fn force_clear(&mut self, range: MemoryRange) {
        self.stack
            .iter_mut()
            .skip(range.start)
            .take(range.len())
            .for_each(|b| *b = 0);

        if let Some(range) = range.relative_to(&self.heap_range()) {
            self.heap
                .iter_mut()
                .skip(range.start)
                .take(range.len())
                .for_each(|b| *b = 0);
        }
    }

    /// Zero memory, performing ownership checks and max access size check.
    pub fn try_clear(&mut self, owner: OwnershipRegisters, range: MemoryRange) -> Result<(), RuntimeError> {
        if range.len() > MEM_MAX_ACCESS_SIZE as usize || !owner.has_ownership_range(&range) {
            return Err(PanicReason::MemoryOverflow.into());
        }

        self.force_clear(range);
        Ok(())
    }

    /// Get a write access to a memory region, without checking for ownership.
    /// Panics on incorrect memory access.
    pub fn force_mut_range(&mut self, range: MemoryRange) -> &mut [u8] {
        if range.end > MEM_SIZE {
            panic!("BUG! Invalid memory access");
        }

        let in_stack = range.relative_to(&self.stack_range());
        let in_heap = range.relative_to(&self.heap_range());

        assert!(in_stack.is_some() != in_heap.is_some(), "BUG! Invalid memory access");

        if let Some(dst) = in_stack {
            &mut self.stack[dst.as_usizes()]
        } else if let Some(dst) = in_heap {
            &mut self.heap[dst.as_usizes()]
        } else {
            unreachable!("Writable range must be fully in stack or heap, as checked above");
        }
    }

    /// Get a write access to a memory region.
    pub fn mut_range(&mut self, owner: OwnershipRegisters, range: MemoryRange) -> Result<&mut [u8], RuntimeError> {
        if range.end > MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        if !owner.has_ownership_range(&range) {
            return Err(PanicReason::MemoryOwnership.into());
        }

        Ok(self.force_mut_range(range))
    }

    /// Write a constant size byte array to the memory, without performing ownership checks.
    pub fn force_write_bytes<const S: usize>(&mut self, addr: usize, data: &[u8; S]) {
        let range = MemoryRange::try_new_usize(addr, S).expect("Bug! Invalid memory access");
        self.force_mut_range(range).copy_from_slice(&data[..]);
    }

    /// Write a constant size byte array to the memory
    pub fn write_bytes<const S: usize>(
        &mut self,
        owner: OwnershipRegisters,
        addr: usize,
        data: &[u8; S],
    ) -> Result<(), RuntimeError> {
        let range = MemoryRange::try_new_usize(addr, S)?;
        self.mut_range(owner, range)?.copy_from_slice(&data[..]);
        Ok(())
    }

    /// Write a single byte
    pub fn set_at(&mut self, owner: OwnershipRegisters, addr: usize, value: u8) -> Result<(), RuntimeError> {
        self.write_bytes(owner, addr, &[value; 1])
    }

    /// Write a constant size byte array to the memory, without performing ownership checks.
    pub fn force_write_slice(&mut self, addr: usize, data: &[u8]) {
        let range = MemoryRange::try_new_usize(addr, data.len()).expect("Bug! Invalid memory access");
        self.force_mut_range(range).copy_from_slice(data);
    }

    /// Write a constant size byte array to the memory
    pub fn write_slice(&mut self, owner: OwnershipRegisters, addr: usize, data: &[u8]) -> Result<(), RuntimeError> {
        let range = MemoryRange::try_new_usize(addr, data.len())?;
        self.mut_range(owner, range)?.copy_from_slice(data);
        Ok(())
    }

    /// Copy bytes from one location to another.
    /// Ownership and max access size checks are performed.
    /// Also fails if the ranges overlap.
    pub fn try_copy_within(
        &mut self,
        owner: OwnershipRegisters,
        dst: usize,
        src: usize,
        len: usize,
    ) -> Result<(), RuntimeError> {
        let dst_range = MemoryRange::try_new_usize(dst, len)?;
        let src_range = MemoryRange::try_new_usize(src, len)?;

        if len > (MEM_MAX_ACCESS_SIZE as usize)
            || dst_range.overlap_with(&src_range).is_some()
            || !owner.has_ownership_range(&dst_range)
        {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            // TODO: optimize, since the ranges do not overlap
            let data: Vec<u8> = self.read_range(src_range).expect("checked").copied().collect();
            let target = self.force_mut_range(dst_range);
            target.copy_from_slice(&data);
            Ok(())
        }
    }
}

impl PartialEq for VmMemory {
    fn eq(&self, other: &Self) -> bool {
        self.read(0, MEM_SIZE)
            .unwrap()
            .zip(other.read(0, MEM_SIZE).unwrap())
            .all(|(a, b)| a == b)
    }
}

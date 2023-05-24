#[cfg(test)]
mod allocation_tests;
mod ownership;
mod range;
#[cfg(test)]
mod tests;

pub use self::ownership::OwnershipRegisters;
pub use self::range::MemoryRange;

use std::{io, iter};

use derivative::Derivative;
use fuel_asm::PanicReason;
use fuel_types::fmt_truncated_hex;

use crate::{consts::MEM_SIZE, prelude::RuntimeError};

/// Page size, in bytes.
pub const VM_PAGE_SIZE: usize = 16 * (1 << 10); // 16 KiB

/// A single page of memory.
pub type MemoryPage = [u8; VM_PAGE_SIZE];

/// A zeroed page of memory.
pub const ZERO_PAGE: MemoryPage = [0u8; VM_PAGE_SIZE];

static_assertions::const_assert!(VM_PAGE_SIZE < MEM_SIZE);
static_assertions::const_assert!(MEM_SIZE % VM_PAGE_SIZE == 0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PageIndex(usize);

impl PageIndex {
    /// The page just past the last byte of memory.
    const ZERO: Self = Self(0);

    /// The page just past the last byte of memory.
    const LIMIT: Self = Self::from_addr(MEM_SIZE).0;

    /// Returns page index of an address, and the offset on that page
    const fn from_addr(addr: usize) -> (Self, usize) {
        (Self(addr / VM_PAGE_SIZE), addr % VM_PAGE_SIZE)
    }
}

/// The memory of a single VM instance.
/// Note that even though the memory is divided into stack and heap pages,
/// those names are only descriptive and do not imply any special behavior.
/// When doing reads, both stack and heap pages are treated the same.
#[derive(Clone, Derivative)]
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

    fn unallocated(&self) -> usize {
        MEM_SIZE
            .checked_sub(self.stack.len() + self.heap.len())
            .expect("Memory over allocated")
    }

    fn is_all_memory_allocated(&self) -> bool {
        self.unallocated() == 0
    }

    /// Attempts to allocate a page of memory from the stack.
    /// Does nothing if all memory is already allocated.
    fn alloc_stack_page(&mut self) {
        if !self.is_all_memory_allocated() {
            self.stack.extend(&ZERO_PAGE);
        }
    }

    /// Attempts to allocate a page of memory from the heap.
    /// Does nothing if all memory is already allocated.
    fn alloc_heap_page(&mut self) {
        if !self.is_all_memory_allocated() {
            self.heap.extend(&ZERO_PAGE);
        }
    }

    fn iter(&self) -> impl Iterator<Item = &u8> + '_ {
        self.stack
            .iter()
            .chain(iter::repeat(&0u8).take(self.unallocated()))
            .chain(self.heap.iter())
    }

    pub fn read(&self, addr: usize, count: usize) -> Result<impl Iterator<Item = &u8> + '_, RuntimeError> {
        let end = addr.saturating_add(count).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        Ok(self.iter().skip(addr).take(count))
    }

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

    /// Zero memory memory without performing ownership checks.
    /// TODO: better name
    pub fn clear_unchecked(&self, addr: usize, len: usize) -> Result<(), RuntimeError> {
        let end = addr.saturating_add(len).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        for dst in self
            .stack
            .iter_mut()
            .chain(iter::repeat_with(|| &mut 0u8).take(self.unallocated()))
            .chain(self.heap.iter_mut())
            .skip(addr)
            .take(len)
        {
            *dst = 0;
        }

        Ok(())
    }

    /// Zero memory memory, performing ownership checks.
    pub fn try_clear(&self, ownership: OwnershipRegisters, addr: usize, len: usize) -> Result<(), RuntimeError> {
        let end = addr.saturating_add(len).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        // TODO: ownership checks

        self.clear_unchecked(addr, len).unwrap();
        Ok(())
    }

    /// Write a constant size byte array to the memory without performing ownership checks.
    /// TODO: better name
    pub fn write_bytes_unchecked<const S: usize>(&self, addr: usize, data: &[u8; S]) -> Result<(), RuntimeError> {
        let end = addr.saturating_add(S).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        // let mut it = self.page_range(s_page..=e_page);

        todo!("memory allocation on write");

        // Ok(result)
    }

    /// Write a constant size byte array to the memory.
    /// This only allows writes to existing pages.
    pub fn write_bytes<const S: usize>(
        &self,
        ownership: OwnershipRegisters,
        addr: usize,
        data: &[u8; S],
    ) -> Result<(), RuntimeError> {
        let end = addr.saturating_add(S).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        todo!("memory allocation on write");
    }

    /// Write a constant size byte array to the memory.
    /// No ownership checks are performed.
    /// TODO: rename
    pub fn write_unchecked(&self, addr: usize, data: &[u8]) -> Result<(), RuntimeError> {
        let end = addr.saturating_add(data.len()).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        todo!("memory allocation on write");
    }

    /// Attempt writing bytes, performing ownership checks first.
    pub fn try_write(&self, ownership: OwnershipRegisters, addr: usize, data: &[u8]) -> Result<(), RuntimeError> {
        let end = addr.saturating_add(data.len()).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        // TODO: ownership checks

        todo!("memory allocation on write");
    }
}

impl PartialEq for VmMemory {
    fn eq(&self, other: &Self) -> bool {
        todo!();
    }
}

#[cfg(test)]
mod allocation_tests;
mod ownership;
mod range;
#[cfg(test)]
mod tests;

pub use self::ownership::OwnershipRegisters;
pub use self::range::MemoryRange;

use std::{io, ops::RangeInclusive};

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
    /// Stack pages are allocated in from the beginning of the address space.
    // #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))]
    stack_pages: Vec<MemoryPage>,
    /// Heap pages are allocated in from the end of the address space, in reverse order.
    // #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))]
    heap_pages: Vec<MemoryPage>,
}
impl VmMemory {
    /// Create a new empty VM memory instance.
    pub const fn new() -> Self {
        Self {
            stack_pages: Vec::new(),
            heap_pages: Vec::new(),
        }
    }

    fn heap_start_page(&self) -> PageIndex {
        PageIndex(MEM_SIZE / VM_PAGE_SIZE - self.heap_pages.len())
    }

    fn is_all_memory_allocated(&self) -> bool {
        self.stack_pages.len() + self.heap_pages.len() == MEM_SIZE / VM_PAGE_SIZE
    }

    /// Attempts to allocate a page of memory from the stack.
    /// Does nothing if all memory is already allocated.
    fn alloc_stack_page(&mut self) {
        if !self.is_all_memory_allocated() {
            self.stack_pages.push(ZERO_PAGE);
        }
    }

    /// Attempts to allocate a page of memory from the heap.
    /// Does nothing if all memory is already allocated.
    fn alloc_heap_page(&mut self) {
        if !self.is_all_memory_allocated() {
            self.heap_pages.push(ZERO_PAGE);
        }
    }

    fn page(&self, index: PageIndex) -> &MemoryPage {
        if index.0 < self.stack_pages.len() {
            &self.stack_pages[index.0]
        } else if index > self.heap_start_page() {
            &self.heap_pages[index.0 - self.heap_start_page().0]
        } else {
            &ZERO_PAGE
        }
    }

    /// Get a mutable reference to a page, but only if it's already allocated.
    fn page_mut(&self, index: PageIndex) -> Option<&mut MemoryPage> {
        if index.0 < self.stack_pages.len() {
            Some(&mut self.stack_pages[index.0])
        } else if index > self.heap_start_page() {
            Some(&mut self.heap_pages[index.0 - self.heap_start_page().0])
        } else {
            None
        }
    }

    fn iter_pages(&self, start_index: PageIndex, count: usize) -> impl Iterator<Item = &MemoryPage> {
        (start_index.0..start_index.0 + count).map(move |i| self.page(PageIndex(i)))
    }

    fn page_range(&self, range: RangeInclusive<PageIndex>) -> impl DoubleEndedIterator<Item = &MemoryPage> {
        (range.start().0..=range.end().0).map(move |i| self.page(PageIndex(i)))
    }

    fn all_pages(&self) -> impl DoubleEndedIterator<Item = &MemoryPage> {
        (0..PageIndex::LIMIT.0).map(move |i| self.page(PageIndex(i)))
    }

    pub fn read_into<W: io::Write>(&self, addr: usize, count: usize, mut target: W) -> Result<(), RuntimeError> {
        let end = addr.saturating_add(count).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        if count == 0 {
            return Ok(());
        }

        let (s_page, s_offset) = PageIndex::from_addr(addr);
        let (e_page, e_offset) = PageIndex::from_addr(end);

        let mut it = self.page_range(s_page..=e_page);

        // Special case: the read is within a single page
        if s_page == e_page {
            target.write_all(&self.page(s_page)[s_offset..e_offset])?;
            return Ok(());
        }

        let mut cursor = 0;

        // The first page must be handled separately, as it may be only partially read
        if let Some(page) = it.next() {
            target.write_all(&page[s_offset..])?;
        }

        // The last page must be handled separately, as it may be only partially read
        let last_page = it.next_back();

        // The rest of the pages are read fully
        for page in it {
            target.write_all(page)?;
        }

        if let Some(page) = last_page {
            target.write_all(&page[..e_offset])?;
        }

        Ok(())
    }

    /// Read a constant size byte array from the memory.
    /// This operation copies the data and is intended for small reads only.
    pub fn read_bytes<const S: usize>(&self, addr: usize) -> Result<[u8; S], RuntimeError> {
        let end = addr.saturating_add(S).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (s_page, s_offset) = PageIndex::from_addr(addr);
        let (e_page, e_offset) = PageIndex::from_addr(end);

        let mut it = self.page_range(s_page..=e_page);

        let mut result = [0u8; S];

        // Special case: the read is within a single page
        if s_page == e_page {
            result.copy_from_slice(&self.page(s_page)[s_offset..e_offset]);
            return Ok(result);
        }

        let mut cursor = 0;

        // The first page must be handled separately, as it may be only partially read
        if let Some(page) = it.next() {
            result[cursor..].copy_from_slice(&page[s_offset..]);
            cursor += VM_PAGE_SIZE - s_offset;
        }

        // The last page must be handled separately, as it may be only partially read
        if let Some(page) = it.next_back() {
            result[e_offset..].copy_from_slice(&page[..e_offset]);
        }

        // The rest of the pages are read fully
        for page in it {
            result[cursor..cursor + VM_PAGE_SIZE].copy_from_slice(page);
            cursor += VM_PAGE_SIZE;
        }

        debug_assert_eq!(cursor + e_offset, S);

        Ok(result)
    }

    /// Write a constant size byte array to the memory without performing ownership checks.
    /// TODO: better name
    pub fn write_bytes_unchecked<const S: usize>(&self, addr: usize, data: &[u8; S]) -> Result<(), RuntimeError> {
        let end = addr.saturating_add(S).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (s_page, s_offset) = PageIndex::from_addr(addr);
        let (e_page, e_offset) = PageIndex::from_addr(end);

        // let mut it = self.page_range(s_page..=e_page);

        todo!("memory allocation on write");

        // // Special case: the write is within a single page
        // if s_page == e_page {
        //     self.page(s_page)[s_offset..e_offset].copy_from_slice(&data[..]);
        //     return Ok(());
        // }

        // let mut cursor = 0;

        // // The first page must be handled separately, as it may be only partially written
        // if let Some(page) = it.next() {
        //     page[cursor..].copy_from_slice(&page[s_offset..]);
        //     cursor += VM_PAGE_SIZE - s_offset;
        // }

        // // The last page must be handled separately, as it may be only partially written
        // if let Some(page) = it.next_back() {
        //     page[e_offset..].copy_from_slice(&page[..e_offset]);
        // }

        // // The rest of the pages are written fully
        // for page in it {
        //     page[cursor..cursor + VM_PAGE_SIZE].copy_from_slice(page);
        //     cursor += VM_PAGE_SIZE;
        // }

        // debug_assert_eq!(cursor + e_offset, S);

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

        let (s_page, s_offset) = PageIndex::from_addr(addr);
        let (e_page, e_offset) = PageIndex::from_addr(end);

        // let mut it = self.page_range(s_page..=e_page);

        todo!("memory allocation on write");

        // // Special case: the write is within a single page
        // if s_page == e_page {
        //     self.page(s_page)[s_offset..e_offset].copy_from_slice(&data[..]);
        //     return Ok(());
        // }

        // let mut cursor = 0;

        // // The first page must be handled separately, as it may be only partially written
        // if let Some(page) = it.next() {
        //     page[cursor..].copy_from_slice(&page[s_offset..]);
        //     cursor += VM_PAGE_SIZE - s_offset;
        // }

        // // The last page must be handled separately, as it may be only partially written
        // if let Some(page) = it.next_back() {
        //     page[e_offset..].copy_from_slice(&page[..e_offset]);
        // }

        // // The rest of the pages are written fully
        // for page in it {
        //     page[cursor..cursor + VM_PAGE_SIZE].copy_from_slice(page);
        //     cursor += VM_PAGE_SIZE;
        // }

        // debug_assert_eq!(cursor + e_offset, S);

        // Ok(result)
    }

    /// Write a constant size byte array to the memory.
    /// No ownership checks are performed.
    /// TODO: rename
    pub fn write_unchecked(&self, addr: usize, data: &[u8]) -> Result<(), RuntimeError> {
        let end = addr.saturating_add(data.len()).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (s_page, s_offset) = PageIndex::from_addr(addr);
        let (e_page, e_offset) = PageIndex::from_addr(end);

        // let mut it = self.page_range(s_page..=e_page);

        todo!("memory allocation on write");
    }

    /// Attempt writing bytes, performing ownership checks first.
    pub fn try_write(&self, ownership: OwnershipRegisters, addr: usize, data: &[u8]) -> Result<(), RuntimeError> {
        let end = addr.saturating_add(data.len()).saturating_sub(1);

        if end >= MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (s_page, s_offset) = PageIndex::from_addr(addr);
        let (e_page, e_offset) = PageIndex::from_addr(end);

        // let mut it = self.page_range(s_page..=e_page);

        // TODO: ownership checks

        todo!("memory allocation on write");
    }
}

impl PartialEq for VmMemory {
    fn eq(&self, other: &Self) -> bool {
        for (a, b) in self.all_pages().zip(other.all_pages()) {
            if a != b {
                return false;
            }
        }

        true
    }
}

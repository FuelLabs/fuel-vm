mod operations;
mod ownership;
mod range;

#[cfg(test)]
mod tests;

pub use self::ownership::OwnershipRegisters;
pub use self::range::MemoryRange;

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

/// Number of new pages allocated by a memory allocation request.
#[derive(Debug, Clone, Copy, PartialEq)]
#[must_use = "Gas charging is required when new pages are allacted"]
pub struct AllocatedPages(pub usize);
impl AllocatedPages {
    /// Returns the cost of allocated pages, or `None` if no pages were allocated.
    pub fn maybe_cost(self, cost_per_page: Word) -> Option<Word> {
        if self.0 == 0 {
            None
        } else {
            // If this ends up saturating, then we'll be out of gas anyway
            Some((self.0 as Word).saturating_mul(cost_per_page))
        }
    }
}

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

    /// Reset the memory to its initial state. This doesn't deallocate the memory buffers.
    pub fn reset(&mut self) {
        self.stack.clear();
        self.heap.clear();
    }

    /// Allocates full memory range for stack, essentially disabling ownership checks.
    /// This is only used for testing.
    #[cfg(test)]
    pub fn fully_allocated() -> Self {
        let mut mem = Self::new();
        let _ = mem.update_allocations(VM_MAX_RAM, VM_MAX_RAM).unwrap();
        mem
    }

    /// Allocates full memory range for stack and fills it with given byte.
    #[cfg(test)]
    pub fn fully_filled(byte: u8) -> Self {
        let mut mem = Self::new();
        let _ = mem.update_allocations(VM_MAX_RAM, VM_MAX_RAM).unwrap();
        mem.stack.fill(byte);
        mem
    }

    /// Returns the number of unallocated bytes.
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

    /// Stack area as a memory range.
    fn stack_range(&self) -> MemoryRange {
        MemoryRange::try_new(0, self.stack.len()).unwrap()
    }

    /// Heap area as a memory range.
    fn heap_range(&self) -> MemoryRange {
        // Never wraps, heap isn't larger than the address space
        let heap_start = MEM_SIZE - self.heap.len();
        MemoryRange::try_new(heap_start, self.heap.len()).unwrap()
    }

    /// Read given number of bytes of memory at address.
    /// Panics on invalid memory access.
    pub fn read(&self, range: &MemoryRange) -> &[u8] {
        let in_stack = range.relative_to(&self.stack_range());
        let in_heap = range.relative_to(&self.heap_range());

        debug_assert!(!(in_stack.is_some() && in_heap.is_some()));

        if let Some(dst) = in_stack {
            &self.stack[dst.as_usizes()]
        } else if let Some(dst) = in_heap {
            &self.heap[dst.as_usizes()]
        } else {
            panic!("Invalid memory read");
        }
    }

    /// Mutable reference at address.
    /// Panics on invalid memory access.
    pub fn write(&mut self, range: &MemoryRange) -> &mut [u8] {
        let in_stack = range.relative_to(&self.stack_range());
        let in_heap = range.relative_to(&self.heap_range());

        debug_assert!(!(in_stack.is_some() && in_heap.is_some()));

        if let Some(dst) = in_stack {
            &mut self.stack[dst.as_usizes()]
        } else if let Some(dst) = in_heap {
            &mut self.heap[dst.as_usizes()]
        } else {
            panic!("Invalid memory write");
        }
    }

    /// Reads a fixed-size array of bytes.
    /// Panics on invalid memory access.
    pub fn read_bytes<A: ToAddr, const LEN: usize>(&self, addr: A) -> [u8; LEN] {
        let range = MemoryRange::try_new(addr, LEN).expect("Invalid memory read");
        let mut buf = [0u8; LEN];
        buf.copy_from_slice(self.read(&range));
        buf
    }

    /// Writes a slice of bytes.
    /// Panics on invalid memory access.
    pub fn write_slice<A: ToAddr>(&mut self, addr: A, data: &[u8]) {
        let range = MemoryRange::try_new(addr, data.len()).expect("Invalid memory write");
        self.write(&range).copy_from_slice(data);
    }

    /// Writes a fixed-size array of bytes.
    /// Panics on invalid memory access.
    pub fn write_bytes<A: ToAddr, const LEN: usize>(&mut self, addr: A, data: &[u8; LEN]) {
        self.write_slice(addr, data)
    }

    /// Copy bytes from one location to another.
    /// Fails if the ranges overlap.
    /// Panics if ranges have different lengths.
    pub fn try_copy_within(&mut self, dst_range: &MemoryRange, src_range: &MemoryRange) -> Result<(), RuntimeError> {
        assert!(dst_range.len() == src_range.len());

        if dst_range.overlap_with(src_range).is_some() {
            return Err(PanicReason::MemoryWriteOverlap.into());
        }

        // TODO: optimize, since the ranges do not overlap
        let data: Vec<u8> = self.read(src_range).to_vec();
        let target = self.write(dst_range);
        target.copy_from_slice(&data);
        Ok(())
    }
}

impl PartialEq for VmMemory {
    fn eq(&self, other: &Self) -> bool {
        self.stack == other.stack && self.heap == other.heap
    }
}
pub type MemoryAddr = usize;

/// Allows taking multiple input types for memory operations.
/// This can be used for both addresses and lenghts in it.
/// Only checks that the type conversion is lossless.
pub trait ToAddr: Copy {
    /// Convert to a raw address, or return None if the conversion is not possible.
    fn to_raw_address(&self) -> Option<MemoryAddr>;
}

impl ToAddr for MemoryAddr {
    fn to_raw_address(&self) -> Option<MemoryAddr> {
        Some(*self)
    }
}

impl ToAddr for Word {
    fn to_raw_address(&self) -> Option<MemoryAddr> {
        (*self).try_into().ok()
    }
}

/// Integer literals
impl ToAddr for i32 {
    fn to_raw_address(&self) -> Option<MemoryAddr> {
        (*self).try_into().ok()
    }
}

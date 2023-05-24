use super::{
    super::{ExecutableTransaction, Interpreter},
    OwnershipRegisters,
};
use crate::consts::*;

use fuel_asm::RegId;
use fuel_types::Word;

use std::ops;

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

    /// Return `true` if the ranges overlap
    pub fn overlaps(&self, other: &Self) -> bool {
        // TODO: tests
        self.start().max(other.start()) < self.end().max(other.end())
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

pub(crate) const fn is_stack_address(sp: &u64, a: Word) -> bool {
    a < *sp
}

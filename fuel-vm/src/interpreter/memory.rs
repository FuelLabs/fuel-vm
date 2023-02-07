use super::{ExecutableTransaction, Interpreter};
use crate::error::RuntimeError;
use crate::{consts::*, context::Context};

use fuel_asm::{PanicReason, RegId};
use fuel_types::{RegisterId, Word};

use std::{ops, ptr};

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

        let heap = vm.registers()[RegId::HP].saturating_add(1);

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

    /// Copy `data` into `addr[..|data|[`
    ///
    /// Check for overflow and memory ownership
    ///
    /// # Panics
    ///
    /// Will panic if data overlaps with `addr[..|data|[`
    pub(crate) fn try_mem_write(&mut self, addr: usize, data: &[u8]) -> Result<(), RuntimeError> {
        let registers = self.ownership_registers();
        try_mem_write(addr, data, registers, &mut self.memory)
    }

    pub(crate) fn try_zeroize(&mut self, addr: usize, len: usize) -> Result<(), RuntimeError> {
        let registers = self.ownership_registers();
        try_zeroize(addr, len, registers, &mut self.memory)
    }

    /// Grant ownership of the range `[a..ab[`
    pub(crate) fn has_ownership_range(&self, range: &MemoryRange) -> bool {
        self.ownership_registers().has_ownership_range(range)
    }

    pub(crate) fn has_ownership_stack(&self, a: Word) -> bool {
        self.ownership_registers().has_ownership_stack(a)
    }

    pub(crate) fn has_ownership_heap(&self, a: Word) -> bool {
        self.ownership_registers().has_ownership_heap(a)
    }

    pub(crate) fn stack_pointer_overflow<F>(&mut self, f: F, v: Word) -> Result<(), RuntimeError>
    where
        F: FnOnce(Word, Word) -> (Word, bool),
    {
        let (result, overflow) = f(self.registers[RegId::SP], v);

        if overflow || result > self.registers[RegId::HP] {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            self.registers[RegId::SP] = result;

            self.inc_pc()
        }
    }

    pub(crate) fn load_byte(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;

        let bc = b.saturating_add(c) as usize;

        if bc >= VM_MAX_RAM as RegisterId {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            self.registers[ra] = self.memory[bc] as Word;

            self.inc_pc()
        }
    }

    pub(crate) fn load_word(&mut self, ra: RegisterId, b: Word, c: Word) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;

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
        let (result, overflow) = self.registers[RegId::HP].overflowing_sub(a);

        if overflow || result < self.registers[RegId::SP] {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            self.registers[RegId::HP] = result;

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
        Self::is_register_writable(ra)?;

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
        let (a, ab) = range.boundaries(self);

        let a_is_stack = a < self.sp;
        let a_is_heap = a > self.hp;

        let ab_is_stack = ab <= self.sp;
        let ab_is_heap = ab >= self.hp;

        a < ab
            && (a_is_stack && ab_is_stack && self.has_ownership_stack(a) && self.has_ownership_stack_exclusive(ab)
                || a_is_heap && ab_is_heap && self.has_ownership_heap(a) && self.has_ownership_heap_exclusive(ab))
    }

    pub(crate) const fn has_ownership_stack(&self, a: Word) -> bool {
        a <= VM_MAX_RAM && self.ssp <= a && a < self.sp
    }

    pub(crate) const fn has_ownership_stack_exclusive(&self, a: Word) -> bool {
        a <= VM_MAX_RAM && self.ssp <= a && a <= self.sp
    }

    pub(crate) fn has_ownership_heap(&self, a: Word) -> bool {
        // TODO implement fp->hp and (addr, size) validations
        // fp->hp
        // it means $hp from the previous context, i.e. what's saved in the
        // "Saved registers from previous context" of the call frame at
        // $fp`
        let external = self.context.is_external();

        self.hp < a && (external && a < VM_MAX_RAM || !external && a <= self.prev_hp)
    }

    pub(crate) fn has_ownership_heap_exclusive(&self, a: Word) -> bool {
        // TODO reflect the pending changes from `has_ownership_heap`
        let external = self.context.is_external();

        self.hp < a
            && (external && a <= VM_MAX_RAM || !external && self.prev_hp.checked_add(1).map_or(false, |f| a <= f))
    }
}

fn try_mem_write(
    addr: usize,
    data: &[u8],
    registers: OwnershipRegisters,
    memory: &mut [u8],
) -> Result<(), RuntimeError> {
    let ax = addr.checked_add(data.len()).ok_or(PanicReason::ArithmeticOverflow)?;

    let range = (ax <= VM_MAX_RAM as usize)
        .then(|| MemoryRange::new(addr as Word, data.len() as Word))
        .ok_or(PanicReason::MemoryOverflow)?;

    registers
        .has_ownership_range(&range)
        .then(|| {
            let src = data.as_ptr();
            let dst = &mut memory[addr] as *mut u8;

            unsafe {
                ptr::copy_nonoverlapping(src, dst, data.len());
            }
        })
        .ok_or_else(|| PanicReason::MemoryOwnership.into())
}

fn try_zeroize(addr: usize, len: usize, registers: OwnershipRegisters, memory: &mut [u8]) -> Result<(), RuntimeError> {
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

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::*;
    use crate::checked_transaction::Checked;
    use crate::prelude::*;
    use fuel_asm::op;
    use fuel_tx::Script;
    use test_case::test_case;

    #[test]
    fn memcopy() {
        let mut vm = Interpreter::with_memory_storage();
        let params = ConsensusParameters::default().with_max_gas_per_tx(Word::MAX / 2);
        let tx = Transaction::script(
            0,
            params.max_gas_per_tx,
            0,
            op::ret(0x10).to_bytes().to_vec(),
            vec![],
            vec![],
            vec![],
            vec![],
        );

        let tx = tx
            .into_checked(Default::default(), &params, vm.gas_costs())
            .expect("default tx should produce a valid checked transaction");

        vm.init_script(tx).expect("Failed to init VM");

        let alloc = 1024;

        // r[0x10] := 1024
        vm.instruction(op::addi(0x10, RegId::ZERO, alloc)).unwrap();
        vm.instruction(op::aloc(0x10)).unwrap();

        // r[0x20] := 128
        vm.instruction(op::addi(0x20, 0x20, 128)).unwrap();

        for i in 0..alloc {
            vm.instruction(op::addi(0x21, RegId::ZERO, i)).unwrap();
            vm.instruction(op::sb(RegId::HP, 0x21, (i + 1) as Immediate12)).unwrap();
        }

        // r[0x23] := m[$hp, 0x20] == m[0x12, 0x20]
        vm.instruction(op::meq(0x23, RegId::HP, 0x12, 0x20)).unwrap();

        assert_eq!(0, vm.registers()[0x23]);

        // r[0x12] := $hp + r[0x20]
        vm.instruction(op::add(0x12, RegId::HP, 0x20)).unwrap();
        vm.instruction(op::add(0x12, RegId::ONE, 0x12)).unwrap();

        // Test ownership
        vm.instruction(op::add(0x30, RegId::HP, RegId::ONE)).unwrap();
        vm.instruction(op::mcp(0x30, 0x12, 0x20)).unwrap();

        // r[0x23] := m[0x30, 0x20] == m[0x12, 0x20]
        vm.instruction(op::meq(0x23, 0x30, 0x12, 0x20)).unwrap();

        assert_eq!(1, vm.registers()[0x23]);

        // Assert ownership
        vm.instruction(op::subi(0x24, RegId::HP, 1)).unwrap();
        let ownership_violated = vm.instruction(op::mcp(0x24, 0x12, 0x20));

        assert!(ownership_violated.is_err());

        // Assert no panic on overlapping
        vm.instruction(op::subi(0x25, 0x12, 1)).unwrap();
        let overlapping = vm.instruction(op::mcp(RegId::HP, 0x25, 0x20));

        assert!(overlapping.is_err());
    }

    #[test]
    fn memrange() {
        let m = MemoryRange::from(..1024);
        let m_p = MemoryRange::new(0, 1024);
        assert_eq!(m, m_p);

        let mut vm = Interpreter::with_memory_storage();
        vm.init_script(Checked::<Script>::default()).expect("Failed to init VM");

        let bytes = 1024;
        vm.instruction(op::addi(0x10, RegId::ZERO, bytes as Immediate12))
            .unwrap();
        vm.instruction(op::aloc(0x10)).unwrap();

        let m = MemoryRange::new(vm.registers()[RegId::HP], bytes);
        assert!(!vm.has_ownership_range(&m));

        let m = MemoryRange::new(vm.registers()[RegId::HP] + 1, bytes);
        assert!(vm.has_ownership_range(&m));

        let m = MemoryRange::new(vm.registers()[RegId::HP] + 1, bytes + 1);
        assert!(!vm.has_ownership_range(&m));

        let m = MemoryRange::new(0, bytes).to_heap(&vm);
        assert!(vm.has_ownership_range(&m));

        let m = MemoryRange::new(0, bytes + 1).to_heap(&vm);
        assert!(!vm.has_ownership_range(&m));
    }

    #[test]
    fn stack_alloc_ownership() {
        let mut vm = Interpreter::with_memory_storage();

        vm.init_script(Checked::<Script>::default()).expect("Failed to init VM");

        vm.instruction(op::move_(0x10, RegId::SP)).unwrap();
        vm.instruction(op::cfei(2)).unwrap();

        // Assert allocated stack is writable
        vm.instruction(op::mcli(0x10, 2)).unwrap();
    }

    #[test_case(
        OwnershipRegisters::test(0..0, 0..0, Context::Call{ block_height: 0}), 0..0
        => false ; "empty mem range"
    )]
    #[test_case(
        OwnershipRegisters::test(0..0, 0..0, Context::Script{ block_height: 0}), 0..0
        => false ; "empty mem range (external)"
    )]
    #[test_case(
        OwnershipRegisters::test(0..0, 0..0, Context::Call{ block_height: 0}), 0..1
        => false ; "empty stack and heap"
    )]
    #[test_case(
        OwnershipRegisters::test(0..0, 0..0, Context::Script{ block_height: 0}), 0..1
        => false ; "empty stack and heap (external)"
    )]
    #[test_case(
        OwnershipRegisters::test(0..1, 0..0, Context::Call{ block_height: 0}), 0..1
        => true ; "in range for stack"
    )]
    #[test_case(
        OwnershipRegisters::test(0..1, 0..0, Context::Call{ block_height: 0}), 0..2
        => false; "above stack range"
    )]
    #[test_case(
        OwnershipRegisters::test(0..0, 0..2, Context::Call{ block_height: 0}), 1..2
        => true ; "in range for heap"
    )]
    #[test_case(
        OwnershipRegisters::test(0..2, 1..2, Context::Call{ block_height: 0}), 0..2
        => true ; "crosses stack and heap"
    )]
    #[test_case(
        OwnershipRegisters::test(0..0, 0..0, Context::Script{ block_height: 0}), 1..2
        => true ; "in heap range (external)"
    )]
    #[test_case(
        OwnershipRegisters::test(0..19, 31..100, Context::Script{ block_height: 0}), 20..30
        => false; "between ranges (external)"
    )]
    #[test_case(
        OwnershipRegisters::test(0..19, 31..100, Context::Script{ block_height: 0}), 0..1
        => true; "in stack range (external)"
    )]
    fn test_ownership(reg: OwnershipRegisters, range: Range<u64>) -> bool {
        let range = MemoryRange::new(range.start, range.end - range.start);
        reg.has_ownership_range(&range)
    }

    fn set_index(index: usize, val: u8, mut array: [u8; 100]) -> [u8; 100] {
        array[index] = val;
        array
    }

    #[test_case(
        1, &[],
        OwnershipRegisters::test(0..1, 100..100, Context::Script{ block_height: 0})
        => (false, [0u8; 100]); "External errors when write is empty"
    )]
    #[test_case(
        1, &[],
        OwnershipRegisters::test(0..1, 100..100, Context::Call{ block_height: 0})
        => (false, [0u8; 100]); "Internal errors when write is empty"
    )]
    #[test_case(
        1, &[2],
        OwnershipRegisters::test(0..2, 100..100, Context::Script{ block_height: 0})
        => (true, set_index(1, 2, [0u8; 100])); "External writes to stack"
    )]
    #[test_case(
        98, &[2],
        OwnershipRegisters::test(0..2, 97..100, Context::Script{ block_height: 0})
        => (true, set_index(98, 2, [0u8; 100])); "External writes to heap"
    )]
    #[test_case(
        1, &[2],
        OwnershipRegisters::test(0..2, 100..100, Context::Call { block_height: 0})
        => (true, set_index(1, 2, [0u8; 100])); "Internal writes to stack"
    )]
    #[test_case(
        98, &[2],
        OwnershipRegisters::test(0..2, 97..100, Context::Call { block_height: 0})
        => (true, set_index(98, 2, [0u8; 100])); "Internal writes to heap"
    )]
    #[test_case(
        1, &[2; 50],
        OwnershipRegisters::test(0..40, 100..100, Context::Script{ block_height: 0})
        => (false, [0u8; 100]); "External too large for stack"
    )]
    #[test_case(
        1, &[2; 50],
        OwnershipRegisters::test(0..40, 100..100, Context::Call{ block_height: 0})
        => (false, [0u8; 100]); "Internal too large for stack"
    )]
    #[test_case(
        61, &[2; 50],
        OwnershipRegisters::test(0..0, 60..100, Context::Call{ block_height: 0})
        => (false, [0u8; 100]); "Internal too large for heap"
    )]
    fn test_mem_write(addr: usize, data: &[u8], registers: OwnershipRegisters) -> (bool, [u8; 100]) {
        let mut memory = [0u8; 100];
        let r = try_mem_write(addr, data, registers, &mut memory[..]).is_ok();
        (r, memory)
    }

    #[test_case(
        1, 0,
        OwnershipRegisters::test(0..1, 100..100, Context::Script{ block_height: 0})
        => (false, [1u8; 100]); "External errors when write is empty"
    )]
    #[test_case(
        1, 0,
        OwnershipRegisters::test(0..1, 100..100, Context::Call{ block_height: 0})
        => (false, [1u8; 100]); "Internal errors when write is empty"
    )]
    #[test_case(
        1, 1,
        OwnershipRegisters::test(0..2, 100..100, Context::Script{ block_height: 0})
        => (true, set_index(1, 0, [1u8; 100])); "External writes to stack"
    )]
    #[test_case(
        98, 1,
        OwnershipRegisters::test(0..2, 97..100, Context::Script{ block_height: 0})
        => (true, set_index(98, 0, [1u8; 100])); "External writes to heap"
    )]
    #[test_case(
        1, 1,
        OwnershipRegisters::test(0..2, 100..100, Context::Call { block_height: 0})
        => (true, set_index(1, 0, [1u8; 100])); "Internal writes to stack"
    )]
    #[test_case(
        98, 1,
        OwnershipRegisters::test(0..2, 97..100, Context::Call { block_height: 0})
        => (true, set_index(98, 0, [1u8; 100])); "Internal writes to heap"
    )]
    #[test_case(
        1, 50,
        OwnershipRegisters::test(0..40, 100..100, Context::Script{ block_height: 0})
        => (false, [1u8; 100]); "External too large for stack"
    )]
    #[test_case(
        1, 50,
        OwnershipRegisters::test(0..40, 100..100, Context::Call{ block_height: 0})
        => (false, [1u8; 100]); "Internal too large for stack"
    )]
    #[test_case(
        61, 50,
        OwnershipRegisters::test(0..0, 60..100, Context::Call{ block_height: 0})
        => (false, [1u8; 100]); "Internal too large for heap"
    )]
    fn test_try_zeroize(addr: usize, len: usize, registers: OwnershipRegisters) -> (bool, [u8; 100]) {
        let mut memory = [1u8; 100];
        let r = try_zeroize(addr, len, registers, &mut memory[..]).is_ok();
        (r, memory)
    }
}

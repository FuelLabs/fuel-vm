//! Types to help constrain inputs to functions to only what is used.
use std::ops::Deref;
use std::ops::DerefMut;

use fuel_asm::PanicReason;
use fuel_asm::Word;
use fuel_types::ContractId;

use crate::consts::MEM_SIZE;
use crate::consts::VM_MAX_RAM;
use crate::prelude::Bug;
use crate::prelude::BugId;
use crate::prelude::BugVariant;
use crate::prelude::RuntimeError;

pub mod reg_key;

#[derive(Clone)]
/// A range of memory that has been checked that it fits into the VM memory.
pub struct CheckedMemRange(core::ops::Range<usize>);

#[derive(Clone)]
/// A range of memory that has been checked that it fits into the VM memory.
pub struct CheckedMemConstLen<const LEN: usize>(CheckedMemRange);

#[derive(Clone)]
/// A range of memory that has been checked that it fits into the VM memory.
/// This range can be used to read a value of type `T` from memory.
pub struct CheckedMemValue<T>(CheckedMemRange, core::marker::PhantomData<T>);

impl<T> CheckedMemValue<T> {
    /// Create a new const sized memory range.
    pub fn new<const SIZE: usize>(address: Word) -> Result<Self, RuntimeError> {
        Ok(Self(
            CheckedMemRange::new_const::<SIZE>(address)?,
            core::marker::PhantomData,
        ))
    }

    /// Try to read a value of type `T` from memory.
    pub fn try_from(self, memory: &[u8; MEM_SIZE]) -> Result<T, RuntimeError>
    where
        T: for<'a> TryFrom<&'a [u8]>,
        RuntimeError: for<'a> From<<T as TryFrom<&'a [u8]>>::Error>,
    {
        Ok(T::try_from(&memory[self.0 .0])?)
    }

    /// The start of the range.
    pub fn start(&self) -> usize {
        self.0.start()
    }

    /// The end of the range.
    pub fn end(&self) -> usize {
        self.0.end()
    }

    #[cfg(test)]
    /// Inspect a value of type `T` from memory.
    pub fn inspect(self, memory: &[u8; MEM_SIZE]) -> T
    where
        T: std::io::Write + Default,
    {
        let mut t = T::default();
        t.write_all(&memory[self.0 .0]).unwrap();
        t
    }
}

impl CheckedMemRange {
    const DEFAULT_CONSTRAINT: core::ops::Range<Word> = 0..VM_MAX_RAM;

    /// Create a new const sized memory range.
    pub fn new_const<const SIZE: usize>(address: Word) -> Result<Self, RuntimeError> {
        Self::new(address, SIZE)
    }

    /// Create a new memory range.
    pub fn new(address: Word, size: usize) -> Result<Self, RuntimeError> {
        Self::new_inner(address, size, Self::DEFAULT_CONSTRAINT)
    }

    /// Create a new memory range with a custom constraint.
    /// The min of the constraints end and `VM_MAX_RAM` will be used.
    pub fn new_with_constraint(
        address: Word,
        size: usize,
        constraint: core::ops::Range<Word>,
    ) -> Result<Self, RuntimeError> {
        if constraint.end > VM_MAX_RAM {
            return Err(Bug::new(BugId::ID009, BugVariant::InvalidMemoryConstraint).into());
        }
        Self::new_inner(address, size, constraint.start..constraint.end)
    }

    /// Create a new memory range, checks that the range is not empty
    /// and that it fits into the constraint.
    fn new_inner(address: Word, size: usize, constraint: core::ops::Range<Word>) -> Result<Self, RuntimeError> {
        let (end, of) = (address as usize).overflowing_add(size);
        let range = address as usize..end;
        if of || !constraint.contains(&(range.end as Word)) || range.is_empty() {
            return Err(PanicReason::MemoryOverflow.into());
        }
        Ok(Self(range))
    }

    /// The start of the range.
    pub fn start(&self) -> usize {
        self.0.start
    }

    /// The end of the range.
    pub fn end(&self) -> usize {
        self.0.end
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

impl<const LEN: usize> CheckedMemConstLen<LEN> {
    /// Create a new const sized memory range.
    pub fn new(address: Word) -> Result<Self, RuntimeError> {
        Ok(Self(CheckedMemRange::new_const::<LEN>(address)?))
    }

    /// Create a new memory range with a custom constraint.
    /// Panics if constraints end > `VM_MAX_RAM`.
    pub fn new_with_constraint(address: Word, constraint: core::ops::Range<Word>) -> Result<Self, RuntimeError> {
        assert!(constraint.end <= VM_MAX_RAM, "Constraint end must be <= VM_MAX_RAM.");
        Ok(Self(CheckedMemRange::new_inner(address, LEN, constraint)?))
    }

    /// Get the memory slice for this range.
    pub fn read(self, memory: &[u8; MEM_SIZE]) -> &[u8; LEN] {
        (&memory[self.0 .0])
            .try_into()
            .expect("This is always correct as the address and LEN are checked on construction.")
    }

    /// Get the mutable memory slice for this range.
    pub fn write(self, memory: &mut [u8; MEM_SIZE]) -> &mut [u8; LEN] {
        (&mut memory[self.0 .0])
            .try_into()
            .expect("This is always correct as the address and LEN are checked on construction.")
    }
}

/// Location of an instructing collected during runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructionLocation {
    /// Context, i.e. current contract. None if running a script.
    pub context: Option<ContractId>,
    /// Offset from the IS register
    pub offset: u64,
}

impl<const LEN: usize> Deref for CheckedMemConstLen<LEN> {
    type Target = CheckedMemRange;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const LEN: usize> DerefMut for CheckedMemConstLen<LEN> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

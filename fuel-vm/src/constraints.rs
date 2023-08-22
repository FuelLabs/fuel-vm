//! Types to help constrain inputs to functions to only what is used.
use std::ops::{
    Deref,
    DerefMut,
};

use fuel_asm::Word;
use fuel_types::ContractId;

#[cfg(test)]
use fuel_types::canonical::Deserialize;

use crate::{
    consts::MEM_SIZE,
    prelude::{
        MemoryRange,
        RuntimeError,
    },
};

pub mod reg_key;

/// A range of memory that has been checked that it fits into the VM memory.
#[derive(Clone)]
// TODO: Replace `LEN` constant with a generic object that implements some trait that
// knows  the static size of the generic.
pub struct CheckedMemConstLen<const LEN: usize>(MemoryRange);

/// A range of memory that has been checked that it fits into the VM memory.
/// This range can be used to read a value of type `T` from memory.
#[derive(Clone)]
// TODO: Merge this type with `CheckedMemConstLen`.
pub struct CheckedMemValue<T>(MemoryRange, core::marker::PhantomData<T>);

impl<T> CheckedMemValue<T> {
    /// Create a new const sized memory range.
    pub fn new<const SIZE: usize>(address: Word) -> Result<Self, RuntimeError> {
        Ok(Self(
            MemoryRange::new_const::<_, SIZE>(address)?,
            core::marker::PhantomData,
        ))
    }

    /// Try to read a value of type `T` from memory.
    pub fn try_from(self, memory: &[u8; MEM_SIZE]) -> Result<T, RuntimeError>
    where
        T: for<'a> TryFrom<&'a [u8]>,
        RuntimeError: for<'a> From<<T as TryFrom<&'a [u8]>>::Error>,
    {
        Ok(T::try_from(&memory[self.0.usizes()])?)
    }

    /// The start of the range.
    pub fn start(&self) -> usize {
        self.0.start
    }

    /// The end of the range.
    pub fn end(&self) -> usize {
        self.0.end
    }

    #[cfg(test)]
    /// Inspect a value of type `T` from memory.
    pub fn inspect(self, memory: &[u8; MEM_SIZE]) -> T
    where
        T: Deserialize,
    {
        T::from_bytes(&memory[self.0.usizes()])
            .expect("Inspect failed; invalid value for type")
    }
}

impl<const LEN: usize> CheckedMemConstLen<LEN> {
    /// Create a new const sized memory range.
    pub fn new(address: Word) -> Result<Self, RuntimeError> {
        Ok(Self(MemoryRange::new_const::<_, LEN>(address)?))
    }

    /// Get the memory slice for this range.
    pub fn read(self, memory: &[u8; MEM_SIZE]) -> &[u8; LEN] {
        (&memory[self.0.usizes()]).try_into().expect(
            "This is always correct as the address and LEN are checked on construction.",
        )
    }

    /// Get the mutable memory slice for this range.
    pub fn write(self, memory: &mut [u8; MEM_SIZE]) -> &mut [u8; LEN] {
        (&mut memory[self.0.usizes()]).try_into().expect(
            "This is always correct as the address and LEN are checked on construction.",
        )
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
    type Target = MemoryRange;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const LEN: usize> DerefMut for CheckedMemConstLen<LEN> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

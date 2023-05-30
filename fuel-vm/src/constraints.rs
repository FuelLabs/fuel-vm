//! Types to help constrain inputs to functions to only what is used.

use std::{marker::PhantomData, ops::Deref};

use fuel_asm::PanicReason;
use fuel_types::{ContractId, Word};

use crate::{consts::MEM_SIZE, interpreter::VmMemory, prelude::RuntimeError};

pub mod reg_key;

/// The address is always checked to be within the memory bounds,
/// but can point to a read-only region (i.e. outside stack or heap).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemoryPtr<const SIZE: usize> {
    addr: usize,
}

impl<const SIZE: usize> MemoryPtr<SIZE> {
    /// Create a new pointer from an address, checking that it's valid
    /// and that the sized value fits into vm memory as well.
    pub fn try_new(addr: Word) -> Result<Self, RuntimeError> {
        Self::try_new_usize(addr.try_into().map_err(|_| PanicReason::MemoryOverflow)?)
    }

    /// Create a new pointer from an address, checking that it's valid
    /// and that the sized value fits into vm memory as well.
    pub fn try_new_usize(addr: usize) -> Result<Self, RuntimeError> {
        if addr.saturating_add(SIZE) > MEM_SIZE {
            return Err(PanicReason::MemoryOverflow.into());
        }

        Ok(Self { addr })
    }

    /// TODO: should verify that the size of the type matches,
    /// but the Rust feature for that is still unstable.
    pub fn typed<T>(self) -> TypedMemoryPtr<T, { SIZE }> {
        TypedMemoryPtr {
            ptr: self,
            _phantom: PhantomData,
        }
    }

    /// Get raw address of the pointer
    pub fn as_usize(&self) -> usize {
        self.addr
    }

    /// Reads array of bytes from memory pointed by this pointer
    pub fn read_bytes(&self, memory: &VmMemory) -> [u8; SIZE] {
        memory.read_bytes(self.addr).expect("Unreachable! Checked pointer")
    }
}

/// When stabilized the const parameter should be inferred from the type, but
/// Rust generics that allow that are still unstable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypedMemoryPtr<T, const SIZE: usize> {
    ptr: MemoryPtr<SIZE>,
    _phantom: PhantomData<T>,
}

impl<T, const SIZE: usize> TypedMemoryPtr<T, SIZE>
where
    T: From<[u8; SIZE]>,
{
    /// Reads typed value from memory
    pub fn read(&self, memory: &VmMemory) -> T {
        let bytes = self.ptr.read_bytes(memory);
        T::from(bytes)
    }
}

impl<T, const SIZE: usize> Deref for TypedMemoryPtr<T, { SIZE }> {
    type Target = MemoryPtr<{ SIZE }>;

    fn deref(&self) -> &Self::Target {
        &self.ptr
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

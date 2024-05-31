//! Pool of VM memory instances for reuse.

use crate::interpreter::{
    Memory,
    MemoryInstance,
};

/// Trait for a VM memory pool.
pub trait VmMemoryPool {
    /// The memory instance returned by this pool.
    type Memory: Memory;

    /// Gets a new VM memory instance from the pool.
    /// The returned instance is allowed to call `recycle` when dropped,
    /// in which case the pool must handle gracefully the case of recycling
    /// it by calling `recycle` manually.
    fn get_new(&self) -> Self::Memory;

    /// Recycles a VM memory instance back into the pool.
    fn recycle(&self, mem: Self::Memory);
}

/// Dummy pool that just returns new instance every time.
#[cfg(any(test, feature = "test-helpers"))]
#[derive(Default, Clone)]
pub struct DummyPool;

#[cfg(any(test, feature = "test-helpers"))]
impl VmMemoryPool for DummyPool {
    type Memory = MemoryInstance;

    fn get_new(&self) -> Self::Memory {
        MemoryInstance::new()
    }

    fn recycle(&self, _mem: Self::Memory) {}
}

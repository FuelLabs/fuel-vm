//! Pool of VM memory instances for reuse.

use crate::interpreter::Memory;

/// Trait for a VM memory pool.
pub trait VmMemoryPool {
    /// Gets a new VM memory instance from the pool.
    fn get_new(&self) -> impl AsRef<Memory> + AsMut<Memory>;

    /// Recycles a VM memory instance back into the pool.
    fn recycle(&self, mem: Memory);
}

/// Dummy pool for testing.
#[cfg(any(test, feature = "test-helpers"))]
#[derive(Default, Clone)]
pub struct DummyPool;

#[cfg(any(test, feature = "test-helpers"))]
impl VmMemoryPool for DummyPool {
    fn get_new(&self) -> impl AsRef<Memory> + AsMut<Memory> {
        Memory::new()
    }

    fn recycle(&self, _mem: Memory) {}
}

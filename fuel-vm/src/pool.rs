//! Pool of VM memory instances for reuse.

use crate::interpreter::Memory;

#[cfg(any(test, feature = "test-helpers"))]
use crate::interpreter::MemoryInstance;

/// Trait for a VM memory pool.
pub trait VmMemoryPool: Sync {
    /// The memory instance returned by this pool.
    type Memory: Memory + Send + Sync + 'static;

    /// Gets a new VM memory instance from the pool.
    fn get_new(&self) -> impl core::future::Future<Output = Self::Memory> + Send;
}

/// Dummy pool that just returns new instance every time.
#[cfg(any(test, feature = "test-helpers"))]
#[derive(Default, Clone)]
pub struct DummyPool;

#[cfg(any(test, feature = "test-helpers"))]
impl VmMemoryPool for DummyPool {
    type Memory = MemoryInstance;

    fn get_new(&self) -> impl core::future::Future<Output = Self::Memory> + Send {
        core::future::ready(MemoryInstance::new())
    }
}

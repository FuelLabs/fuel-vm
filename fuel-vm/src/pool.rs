//! Pool of VM memory instances for reuse.

use alloc::{
    sync::Arc,
    vec::Vec,
};
use core::fmt;

use crate::interpreter::Memory;

/// Memory instance originating from a pool.
/// Will be recycled back into the pool when dropped.
pub struct MemoryFromPool {
    pool: VmPool,
    memory: Option<Memory>,
    /// Used only by tests, as Clone isn't implemented otherwise
    #[cfg(any(test, feature = "test-helpers"))]
    original: bool,
}
impl Drop for MemoryFromPool {
    fn drop(&mut self) {
        #[cfg(any(test, feature = "test-helpers"))]
        if !self.original {
            return;
        }
        self.pool
            .recycle(self.memory.take().expect("Instance recycled already"));
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl Clone for MemoryFromPool {
    fn clone(&self) -> Self {
        MemoryFromPool {
            pool: self.pool.clone(),
            memory: self.memory.clone(),
            original: false,
        }
    }
}

impl fmt::Debug for MemoryFromPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryFromPool")
            .field("pool", &"..")
            .field("memory", &self.memory)
            .finish()
    }
}

impl AsRef<Memory> for MemoryFromPool {
    fn as_ref(&self) -> &Memory {
        self.memory.as_ref().expect("Instance recycled already")
    }
}

impl AsMut<Memory> for MemoryFromPool {
    fn as_mut(&mut self) -> &mut Memory {
        self.memory.as_mut().expect("Instance recycled already")
    }
}

/// Pool of VM memory instances for reuse.
#[derive(Default, Clone)]
pub struct VmPool {
    #[cfg(feature = "std")]
    pool: Arc<std::sync::Mutex<Vec<Memory>>>,
    #[cfg(not(feature = "std"))]
    pool: Arc<spin::Mutex<Vec<Memory>>>,
}
impl VmPool {
    /// Gets a new raw VM memory instance from the pool.
    pub fn take_raw(&self) -> Memory {
        #[cfg(feature = "std")]
        let mut pool = self.pool.lock().expect("poisoned");
        #[cfg(not(feature = "std"))]
        let mut pool = self.pool.lock();
        pool.pop().unwrap_or_default()
    }

    /// Gets a new VM memory instance from the pool.
    pub fn get_new(&self) -> MemoryFromPool {
        MemoryFromPool {
            pool: self.clone(),
            memory: Some(self.take_raw()),
            #[cfg(any(test, feature = "test-helpers"))]
            original: true,
        }
    }

    /// Recycles a VM memory instance back into the pool.
    pub fn recycle(&self, mut mem: Memory) {
        mem.reset();
        #[cfg(feature = "std")]
        let mut pool = self.pool.lock().expect("poisoned");
        #[cfg(not(feature = "std"))]
        let mut pool = self.pool.lock();
        pool.push(mem);
    }
}

impl fmt::Debug for VmPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "std")]
        match self.pool.lock() {
            Ok(pool) => write!(f, "VmPool {{ pool: [{} items] }}", pool.len()),
            Err(_) => write!(f, "VmPool {{ pool: [poisoned] }}"),
        }

        #[cfg(not(feature = "std"))]
        write!(f, "VmPool {{ pool: [{} items] }}", self.pool.lock().len())
    }
}

/// A global pool of VM memory instances used to speed up tests.
#[cfg(all(feature = "std", any(test, feature = "test-helpers")))]
static TEST_POOL: std::sync::OnceLock<VmPool> = std::sync::OnceLock::new();

/// Get the global VM pool used for tests and test builders.
/// On no_std targets this returns a dummy pool that reallocates every time.
#[cfg(all(feature = "std", any(test, feature = "test-helpers")))]
pub fn test_pool() -> VmPool {
    TEST_POOL.get_or_init(VmPool::default).clone()
}

/// Get the global VM pool used for tests and test builders.
/// On no_std targets this returns a dummy pool that reallocates every time.
#[cfg(all(not(feature = "std"), any(test, feature = "test-helpers")))]
pub fn test_pool() -> VmPool {
    VmPool::default()
}

#[test]
fn test_vm_pool() {
    let pool = VmPool::default();

    let mut mem_guard = pool.get_new();
    let mem = mem_guard.as_mut();
    mem.grow_stack(1024).expect("Unable to grow stack");
    mem.write_bytes_noownerchecks(0, [1, 2, 3, 4])
        .expect("Unable to write stack");
    let ptr1 = mem.stack_raw() as *const _ as *const u8 as usize;
    drop(mem_guard);

    // Make sure we get the same memory allocation back
    let mem = pool.get_new();
    let ptr2 = mem.as_ref().stack_raw() as *const _ as *const u8 as usize;
    assert_eq!(ptr1, ptr2);
}

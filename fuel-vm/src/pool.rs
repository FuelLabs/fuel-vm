//! Pool of VM memory instances for reuse.

use alloc::{
    sync::Arc,
    vec::Vec,
};
use core::fmt;

use crate::interpreter::{
    Memory,
    VmMemory,
};

/// Pool of VM memory instances for reuse.
#[derive(Default, Clone)]
pub struct VmPool {
    #[cfg(feature = "std")]
    pool: Arc<std::sync::Mutex<Vec<Memory>>>,
    #[cfg(not(feature = "std"))]
    pool: Arc<spin::Mutex<Vec<Memory>>>,
}
impl VmPool {
    /// Gets a new VM memory instance from the pool.
    pub fn get_new(&self) -> Memory {
        #[cfg(feature = "std")]
        let mut pool = self.pool.lock().expect("poisoned");
        #[cfg(not(feature = "std"))]
        let mut pool = self.pool.lock();
        pool.pop().unwrap_or_default()
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

    let mut mem = pool.get_new();
    mem.grow_stack(1024).expect("Unable to grow stack");
    mem.write_bytes_noownerchecks(0, [1, 2, 3, 4])
        .expect("Unable to write stack");
    let ptr1 = mem.stack_raw() as *const _ as *const u8 as usize;
    pool.recycle(mem);

    // Make sure we get the same memory allocation back
    let mem = pool.get_new();
    let ptr2 = mem.stack_raw() as *const _ as *const u8 as usize;
    assert_eq!(ptr1, ptr2);
}

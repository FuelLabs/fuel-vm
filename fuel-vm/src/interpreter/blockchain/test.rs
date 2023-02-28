use crate::context::Context;
use crate::interpreter::memory::Memory;

use super::*;

mod scwq;
mod srwq;
mod swwq;

fn mem(chains: &[&[u8]]) -> Memory<MEM_SIZE> {
    let mut vec: Vec<_> = chains.iter().flat_map(|i| i.iter().copied()).collect();
    vec.resize(MEM_SIZE, 0);
    vec.try_into().unwrap()
}
const fn key(k: u8) -> [u8; 32] {
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, k,
    ]
}

impl OwnershipRegisters {
    pub fn test(stack: Range<u64>, heap: Range<u64>, context: Context) -> Self {
        Self {
            sp: stack.end,
            ssp: stack.start,
            hp: heap.start,
            prev_hp: heap.end,
            context,
        }
    }
}

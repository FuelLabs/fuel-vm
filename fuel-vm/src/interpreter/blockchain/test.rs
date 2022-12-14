use crate::context::Context;

use super::*;

mod scwq;
mod srwq;
mod swwq;

fn mem(chains: &[&[u8]]) -> Vec<u8> {
    let mut vec: Vec<_> = chains.iter().flat_map(|i| i.iter().copied()).collect();
    vec.resize(200, 0);
    vec
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

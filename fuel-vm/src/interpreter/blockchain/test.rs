#![allow(clippy::cast_possible_truncation)]

use core::ops::Range;

use super::*;

impl OwnershipRegisters {
    pub fn test(stack: Range<u64>, heap: Range<u64>) -> Self {
        Self {
            sp: stack.end,
            ssp: stack.start,
            hp: heap.start,
            prev_hp: heap.end,
        }
    }
}

#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

use super::*;
use test_case::test_case;

use crate::error::PanicOrBug;

#[test_case(true, 20, 0, 40, 10 => Ok(()); "Can move sp up")]
#[test_case(false, 20, 0, 40, 10 => Ok(()); "Can move sp down")]
#[test_case(true, u64::MAX - 10, 0, u64::MAX, 20 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "Panics on overflowing addition")]
#[test_case(false, 10, 0, 11, 20 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "Panics on underflowing subtraction")]
#[test_case(false, 0, 0, u64::MAX, 1 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "Panics on zero check for underflowing subtraction")]
#[test_case(false, 8, 8, u64::MAX, 1 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "Panics on sp < ssp")]
fn test_stack_pointer_overflow(
    add: bool,
    mut sp: Word,
    ssp: Word,
    hp: Word,
    v: Word,
) -> SimpleResult<()> {
    let mut memory = MemoryInstance::new();
    let mut pc = 4;
    let old_sp = sp;

    stack_pointer_overflow(
        RegMut::new(&mut sp),
        Reg::new(&ssp),
        Reg::new(&hp),
        RegMut::new(&mut pc),
        if add {
            Word::overflowing_add
        } else {
            Word::overflowing_sub
        },
        v,
        &mut memory,
    )?;

    assert_eq!(pc, 8);
    if add {
        assert_eq!(sp, old_sp + v);
    } else {
        assert_eq!(sp, old_sp - v);
    }
    Ok(())
}

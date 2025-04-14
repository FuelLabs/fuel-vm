#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

use alloc::vec;

use super::*;
use test_case::test_case;

use crate::error::PanicOrBug;

#[test_case(0, 0, 0 => Ok(0))]
#[test_case(0, 0, 1 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "Underflow")]
#[test_case(10, 0, 11 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "Underflow more")]
#[test_case(12, 10, 3 => Err(PanicOrBug::Panic(PanicReason::MemoryGrowthOverlap)); "Into stack")]
#[test_case(10, 10, 0 => Ok(10); "No available memory")]
#[test_case(15, 10, 6 => Err(PanicOrBug::Panic(PanicReason::MemoryGrowthOverlap)); "Insufficient memory")]
#[test_case(20, 10, 0 => Ok(20); "Zero allocation size")]
#[test_case(20, 10, 10 => Ok(10); "Allocation size equal to available memory")]
#[test_case(20, 10, 5 => Ok(15); "Allocation size smaller than available memory")]
fn test_malloc(mut hp: Word, sp: Word, a: Word) -> SimpleResult<Word> {
    let mut memory = MemoryInstance::new();
    memory.hp = hp as usize;
    let mut pc = 4;
    malloc(
        RegMut::new(&mut hp),
        Reg::new(&sp),
        RegMut::new(&mut pc),
        a,
        &mut memory,
    )?;
    assert_eq!(pc, 8);

    Ok(hp)
}

#[test_case(true, 1, 10 => Ok(()); "Can clear some bytes")]
#[test_case(false, 1, 10 => Err(PanicOrBug::Panic(PanicReason::MemoryOwnership)); "No ownership")]
#[test_case(true, 0, 10 => Ok(()); "Memory range starts at 0")]
#[test_case(true, MEM_SIZE as Word - 10, 10 => Ok(()); "Memory range ends at last address")]
#[test_case(true, 1, VM_MAX_RAM + 1 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "Memory range size exceeds limit")]
fn test_memclear(has_ownership: bool, a: Word, b: Word) -> SimpleResult<()> {
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut pc = 4;
    let mut owner = OwnershipRegisters {
        sp: 0,
        ssp: 0,
        hp: MEM_SIZE as Word,
        prev_hp: MEM_SIZE as Word,
    };
    if has_ownership {
        owner.ssp = a;
        owner.sp = a + b;
    }

    memclear(&mut memory, owner, RegMut::new(&mut pc), a, b)?;

    assert_eq!(pc, 8);
    let expected = vec![0u8; b as usize];
    let ab = a.checked_add(b).unwrap();
    assert_eq!(memory[a as usize..ab as usize], expected[..]);

    Ok(())
}

#[test_case(1, 20, 0 => Ok(()); "Can copy zero bytes")]
#[test_case(1, 20, 10 => Ok(()); "Can copy some bytes")]
#[test_case(10, 20, 10 => Ok(()); "Can copy some bytes in close range")]
#[test_case(21, 20, 10 => Err(PanicReason::MemoryWriteOverlap.into()); "b <= a < bc")]
#[test_case(14, 20, 10 => Err(PanicReason::MemoryWriteOverlap.into()); "b < ac <= bc")]
#[test_case(21, 22, 10 => Err(PanicReason::MemoryWriteOverlap.into()); "a <= b < ac")]
#[test_case(21, 20, 10 => Err(PanicReason::MemoryWriteOverlap.into()); "a < bc <= ac")]
fn test_memcopy(a: Word, b: Word, c: Word) -> SimpleResult<()> {
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[b as usize..b as usize + c as usize].copy_from_slice(&vec![2u8; c as usize]);
    let mut pc = 4;
    let owner = OwnershipRegisters::test_full_stack();

    memcopy(&mut memory, owner, RegMut::new(&mut pc), a, b, c)?;

    assert_eq!(pc, 8);
    let expected = vec![2u8; c as usize];
    assert_eq!(memory[a as usize..a as usize + c as usize], expected[..]);

    Ok(())
}

#[test_case(1, 20, 10 => Ok(()); "Can compare some bytes")]
#[test_case(MEM_SIZE as Word, 1, 2 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "b+d > MAX_RAM")]
#[test_case(1, MEM_SIZE as Word, 2 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "c+d > MAX_RAM")]
#[test_case(1, 1, VM_MAX_RAM + 1 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "d > VM_MAX_RAM")]
#[test_case(u64::MAX/2, 1, u64::MAX/2 + 1 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "b+d overflows")]
#[test_case(1, u64::MAX/2, u64::MAX/2 + 1 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)); "c+d overflows")]
#[test_case(0, 0, 0 => Ok(()); "smallest input values")]
#[test_case(0, VM_MAX_RAM/2, VM_MAX_RAM/2 => Ok(()); "maximum range of addressable memory")]
fn test_memeq(b: Word, c: Word, d: Word) -> SimpleResult<()> {
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let r = (b as usize).min(MEM_SIZE)
        ..((b as usize).min(MEM_SIZE) + (d as usize).min(MEM_SIZE)).min(MEM_SIZE);
    memory[r].fill(2u8);
    let r = (c as usize).min(MEM_SIZE)
        ..((c as usize).min(MEM_SIZE) + (d as usize).min(MEM_SIZE)).min(MEM_SIZE);
    memory[r].fill(2u8);
    let mut pc = 4;
    let mut result = 0;

    memeq(&mut memory, &mut result, RegMut::new(&mut pc), b, c, d)?;

    assert_eq!(pc, 8);
    assert_eq!(result, 1);

    Ok(())
}

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

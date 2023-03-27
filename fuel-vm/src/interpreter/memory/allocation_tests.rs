use super::*;
use test_case::test_case;

#[test_case(0, 0, 0 => Ok(0))]
#[test_case(0, 0, 1 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Underflow")]
#[test_case(10, 0, 11 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Underflow more")]
#[test_case(12, 10, 3 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Into stack")]
fn test_malloc(mut hp: Word, sp: Word, a: Word) -> Result<Word, RuntimeError> {
    let mut pc = 4;
    malloc(RegMut::new(&mut hp), Reg::new(&sp), RegMut::new(&mut pc), a)?;
    assert_eq!(pc, 8);

    Ok(hp)
}

#[test_case(true, 1, 10 => Ok(()); "Can clear some bytes")]
fn test_memclear(has_ownership: bool, a: Word, b: Word) -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut pc = 4;
    let mut owner = OwnershipRegisters {
        sp: 0,
        ssp: 0,
        hp: 0,
        prev_hp: 0,
        context: Context::Script {
            block_height: Default::default(),
        },
    };
    if has_ownership {
        owner.ssp = a - 1;
        owner.sp = a + b + 1;
    }

    memclear(&mut memory, owner, RegMut::new(&mut pc), a, b)?;

    assert_eq!(pc, 8);
    let expected = vec![0u8; b as usize - a as usize];
    assert_eq!(memory[a as usize..b as usize], expected[..]);

    Ok(())
}

#[test_case(true, 1, 20, 10 => Ok(()); "Can copy some bytes")]
#[test_case(true, 10, 20, 10 => Ok(()); "Can copy some bytes in close range")]
#[test_case(true, 21, 20, 10 => Err(PanicReason::MemoryOverflow.into()); "b <= a < bc")]
#[test_case(true, 14, 20, 10 => Err(PanicReason::MemoryOverflow.into()); "b < ac <= bc")]
#[test_case(true, 21, 22, 10 => Err(PanicReason::MemoryOverflow.into()); "a <= b < ac")]
#[test_case(true, 21, 20, 10 => Err(PanicReason::MemoryOverflow.into()); "a < bc <= ac")]
fn test_memcopy(has_ownership: bool, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[b as usize..b as usize + c as usize].copy_from_slice(&vec![2u8; c as usize]);
    let mut pc = 4;
    let mut owner = OwnershipRegisters {
        sp: 0,
        ssp: 0,
        hp: 0,
        prev_hp: 0,
        context: Context::Script {
            block_height: Default::default(),
        },
    };
    if has_ownership {
        owner.ssp = b - 1;
        owner.sp = b + c + 1;
    }

    memcopy(&mut memory, owner, RegMut::new(&mut pc), a, b, c)?;

    assert_eq!(pc, 8);
    let expected = vec![2u8; c as usize];
    assert_eq!(memory[a as usize..a as usize + c as usize], expected[..]);

    Ok(())
}

#[test_case(1, 20, 10 => Ok(()); "Can compare some bytes")]
fn test_memeq(b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[b as usize..b as usize + d as usize].copy_from_slice(&vec![2u8; d as usize]);
    memory[c as usize..c as usize + d as usize].copy_from_slice(&vec![2u8; d as usize]);
    let mut pc = 4;
    let mut result = 0;

    memeq(&mut memory, &mut result, RegMut::new(&mut pc), b, c, d)?;

    assert_eq!(pc, 8);
    assert_eq!(result, 1);

    Ok(())
}

#[test_case(true, 20, 40, 10 => Ok(()); "Can move sp up")]
#[test_case(false, 20, 40, 10 => Ok(()); "Can move sp down")]
fn test_stack_pointer_overflow(add: bool, mut sp: Word, hp: Word, v: Word) -> Result<(), RuntimeError> {
    let mut pc = 4;
    let old_sp = sp;

    stack_pointer_overflow(
        RegMut::new(&mut sp),
        Reg::new(&hp),
        RegMut::new(&mut pc),
        if add {
            Word::overflowing_add
        } else {
            Word::overflowing_sub
        },
        v,
    )?;

    assert_eq!(pc, 8);
    if add {
        assert_eq!(sp, old_sp + v);
    } else {
        assert_eq!(sp, old_sp - v);
    }
    Ok(())
}

#[test_case(20, 20 => Ok(()); "Can load a byte")]
fn test_load_byte(b: Word, c: Word) -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[(b + c) as usize] = 2;
    let mut pc = 4;
    let mut result = 0;

    load_byte(&memory, RegMut::new(&mut pc), &mut result, b, c)?;

    assert_eq!(pc, 8);
    assert_eq!(result, 2);

    Ok(())
}

#[test_case(20, 20 => Ok(()); "Can load a word")]
fn test_load_word(b: Word, c: Word) -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    let start = b as usize + (c as usize * 8);
    memory[start..start + 8].copy_from_slice(&[2u8; 8]);
    let mut pc = 4;
    let mut result = 0;

    load_word(&memory, RegMut::new(&mut pc), &mut result, b, c)?;

    assert_eq!(pc, 8);
    assert_eq!(result, Word::from_be_bytes([2u8; 8]));

    Ok(())
}

#[test_case(true, 20, 30, 40 => Ok(()); "Can store a byte")]
fn test_store_byte(has_ownership: bool, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut pc = 4;
    let mut owner = OwnershipRegisters {
        sp: 0,
        ssp: 0,
        hp: 0,
        prev_hp: 0,
        context: Context::Script {
            block_height: Default::default(),
        },
    };
    if has_ownership {
        owner.ssp = b - 1;
        owner.sp = b + c + 1;
    }

    store_byte(&mut memory, owner, RegMut::new(&mut pc), a, b, c)?;

    assert_eq!(pc, 8);
    assert_eq!(memory[(a + c) as usize], b as u8);

    Ok(())
}

#[test_case(true, 20, 30, 40 => Ok(()); "Can store a word")]
fn test_store_word(has_ownership: bool, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut pc = 4;
    let mut owner = OwnershipRegisters {
        sp: 0,
        ssp: 0,
        hp: 0,
        prev_hp: 0,
        context: Context::Script {
            block_height: Default::default(),
        },
    };
    if has_ownership {
        owner.ssp = b - 1;
        owner.sp = b + c + 1;
    }

    store_word(&mut memory, owner, RegMut::new(&mut pc), a, b, c)?;

    assert_eq!(pc, 8);
    let start = (a + c * 8) as usize;
    assert_eq!(memory[start..start + 8], b.to_be_bytes()[..]);

    Ok(())
}

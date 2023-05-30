use crate::{context::Context, constraints::reg_key::{RegMut, Reg}};

use super::*;
use test_case::test_case;

#[test_case(0, 0, 0 => Ok(0))]
#[test_case(0, 0, 1 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Underflow")]
#[test_case(10, 0, 11 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Underflow more")]
#[test_case(12, 10, 3 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Into stack")]
#[test_case(10, 10, 0 => Ok(10); "No available memory")]
#[test_case(15, 10, 6 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Insufficient memory")]
#[test_case(20, 10, 0 => Ok(20); "Zero allocation size")]
#[test_case(20, 10, 10 => Ok(10); "Allocation size equal to available memory")]
#[test_case(20, 10, 5 => Ok(15); "Allocation size smaller than available memory")]
fn test_malloc(mut hp: Word, sp: Word, a: Word) -> Result<Word, RuntimeError> {
    let mut pc = 4;
    malloc(RegMut::new(&mut hp), Reg::new(&sp), RegMut::new(&mut pc), a)?;
    assert_eq!(pc, 8);

    Ok(hp)
}

#[test_case(true, 1, 10 => Ok(()); "Can clear some bytes")]
#[test_case(false, 1, 10 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "No ownership")]
#[test_case(true, 0, 10 => Ok(()); "Memory range starts at 0")]
#[test_case(true, MEM_SIZE as Word - 10, 10 => Ok(()); "Memory range ends at last address")]
#[test_case(true, 1, MEM_MAX_ACCESS_SIZE + 1 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Memory range size exceeds limit")]
fn test_memclear(has_ownership: bool, a: Word, b: Word) -> Result<(), RuntimeError> {
    let mut memory = VmMemory::new();
    let mut pc = 4;
    let mut owner = OwnershipRegisters {
        sp: 0,
        ssp: 0,
        hp: MEM_SIZE as Word,
        prev_hp: MEM_SIZE as Word,
        context: Context::Script {
            block_height: Default::default(),
        },
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

#[test_case(true, 1, 20, 0 => Ok(()); "Can copy zero bytes")]
#[test_case(true, 1, 20, 10 => Ok(()); "Can copy some bytes")]
#[test_case(true, 10, 20, 10 => Ok(()); "Can copy some bytes in close range")]
#[test_case(true, 21, 20, 10 => Err(PanicReason::MemoryOverflow.into()); "b <= a < bc")]
#[test_case(true, 14, 20, 10 => Err(PanicReason::MemoryOverflow.into()); "b < ac <= bc")]
#[test_case(true, 21, 22, 10 => Err(PanicReason::MemoryOverflow.into()); "a <= b < ac")]
#[test_case(true, 21, 20, 10 => Err(PanicReason::MemoryOverflow.into()); "a < bc <= ac")]
fn test_memcopy(has_ownership: bool, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
    let mut memory = VmMemory::fully_allocated();
    memory.force_mut_range(MemoryRange::try_new(b, c).unwrap())
    .copy_from_slice(&vec![2u8; c as usize]);
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
#[test_case(MEM_SIZE as Word, 1, 2 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "b+d > MAX_RAM")]
#[test_case(1, MEM_SIZE as Word, 2 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "c+d > MAX_RAM")]
#[test_case(1, 1, MEM_MAX_ACCESS_SIZE + 1 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "d > MEM_MAX_ACCESS_SIZE")]
#[test_case(u64::MAX/2, 1, u64::MAX/2 + 1 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "b+d overflows")]
#[test_case(1, u64::MAX/2, u64::MAX/2 + 1 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "c+d overflows")]
#[test_case(0, 0, 0 => Ok(()); "smallest input values")]
#[test_case(0, MEM_MAX_ACCESS_SIZE/2, MEM_MAX_ACCESS_SIZE/2 => Ok(()); "maximum range of addressable memory")]
fn test_memeq(b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
    let mut memory = VmMemory::new();
    let r = (b as usize).min(MEM_SIZE)..((b as usize).min(MEM_SIZE) + (d as usize).min(MEM_SIZE)).min(MEM_SIZE);
    memory[r].fill(2u8);
    let r = (c as usize).min(MEM_SIZE)..((c as usize).min(MEM_SIZE) + (d as usize).min(MEM_SIZE)).min(MEM_SIZE);
    memory[r].fill(2u8);
    let mut pc = 4;
    let mut result = 0;

    memeq(&mut memory, &mut result, RegMut::new(&mut pc), b, c, d)?;

    assert_eq!(pc, 8);
    assert_eq!(result, 1);

    Ok(())
}

#[test_case(true, 20, 40, 10 => Ok(()); "Can move sp up")]
#[test_case(false, 20, 40, 10 => Ok(()); "Can move sp down")]
#[test_case(true, u64::MAX - 10, u64::MAX, 20 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Panics on overflowing addition")]
#[test_case(false, 10, 11, 20 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Panics on underflowing subtraction")]
#[test_case(true, u64::MAX, u64::MAX, 0 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Panics on equality check for overflowing addition")]
#[test_case(false, 0, u64::MAX, 1 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Panics on zero check for underflowing subtraction")]
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
#[test_case(0, 0 => Ok(()); "handles memory loading at address 0")]
#[test_case(VM_MAX_RAM + 1, 0 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "b > VM_MAX_RAM")]
#[test_case(VM_MAX_RAM - 1, 0 => Ok(()); "b eq VM_MAX_RAM - 1")]
#[test_case(0, VM_MAX_RAM + 1 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "c > VM_MAX_RAM")]
#[test_case(0, VM_MAX_RAM - 1 => Ok(()); "c eq VM_MAX_RAM - 1")]
#[test_case(u32::MAX as u64, u32::MAX as u64 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "b + c overflow")]
fn test_load_byte(b: Word, c: Word) -> Result<(), RuntimeError> {
    let mut memory = VmMemory::new();
    memory[((b + c) as usize).min(MEM_SIZE - 1)] = 2;
    let mut pc = 4;
    let mut result = 0;

    load_byte(&memory, RegMut::new(&mut pc), &mut result, b, c)?;

    assert_eq!(pc, 8);
    assert_eq!(result, 2);

    Ok(())
}

#[test_case(20, 20 => Ok(()); "Can load a word")]
#[test_case(VM_MAX_RAM, 1 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "b + 8 * c gteq VM_MAX_RAM")]
fn test_load_word(b: Word, c: Word) -> Result<(), RuntimeError> {
    // create a mutable memory with size `MEM_SIZE`
    let mut memory = VmMemory::new();

    // calculate start location where 8 bytes of value will be stored based on `b` and `c` values.
    let start = (b as usize + (c as usize * 8)).min(MEM_SIZE - 8);

    // write 2u8 to a slice of memory (starting at the 'start' location with a length of 8)
    memory[start..start + 8].copy_from_slice(&[2u8; 8]);

    // initialize pc to 4 and result to 0
    let mut pc = 4;
    let mut result = 0;

    // read the memory from the calculated location and store it in `result`, also increment the `pc` by one word(8 bytes).
    load_word(&memory, RegMut::new(&mut pc), &mut result, b, c)?;

    // ensure that `pc` is 8 now and the result matches [2u8; 8] i.e., 2 bytes repeated 8 times.
    assert_eq!(pc, 8);
    assert_eq!(result, Word::from_be_bytes([2u8; 8]));

    Ok(())
}

#[test_case(true, 20, 30, 40 => Ok(()); "Can store a byte")]
#[test_case(false, VM_MAX_RAM - 1, 100, 2 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Memory overflow on heap")]
#[test_case(false, 0, 100, VM_MAX_RAM - 1 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Memory overflow on stack")]
#[test_case(true, VM_MAX_RAM, 1, 1 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Memory overflow by address range")]
fn test_store_byte(has_ownership: bool, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
    let mut memory = VmMemory::new();
    let mut pc = 4;
    let mut owner = OwnershipRegisters {
        sp: 0,
        ssp: 0,
        hp: VM_MAX_RAM,
        prev_hp: VM_MAX_RAM,
        context: Context::Script {
            block_height: Default::default(),
        },
    };
    if has_ownership {
        owner.ssp = b;
        owner.sp = b + c;
    }

    store_byte(&mut memory, owner, RegMut::new(&mut pc), a, b, c)?;

    assert_eq!(pc, 8);
    assert_eq!(memory[(a + c) as usize], b as u8);

    Ok(())
}

#[test_case(true, 20, 30, 40 => Ok(()); "Can store a word")]
#[test_case(true, 20, 30, VM_MAX_RAM => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Fails due to memory overflow")]
#[test_case(false, 20, 30, 40 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Fails due to not having ownership of the range")]
fn test_store_word(has_ownership: bool, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
    let mut memory = VmMemory::new();
    let mut pc = 4;
    let mut owner = OwnershipRegisters {
        sp: 0,
        ssp: 0,
        hp: VM_MAX_RAM,
        prev_hp: VM_MAX_RAM,
        context: Context::Script {
            block_height: Default::default(),
        },
    };
    if has_ownership {
        owner.ssp = b;
        owner.sp = b + (8 * c);
    }

    store_word(&mut memory, owner, RegMut::new(&mut pc), a, b, c)?;

    assert_eq!(pc, 8);
    let start = (a + c * 8) as usize;
    assert_eq!(memory[start..start + 8], b.to_be_bytes()[..]);

    Ok(())
}

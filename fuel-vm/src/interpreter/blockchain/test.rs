use alloc::{
    vec,
    vec::Vec,
};

use core::{
    convert::Infallible,
    ops::Range,
};

use crate::{
    context::Context,
    interpreter::memory::Memory,
    storage::MemoryStorage,
};
use test_case::test_case;

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
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, k,
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

#[test_case(false, 0, None, 32 => Ok((0, 0)); "Nothing set")]
#[test_case(false, 0, 29, 32 => Ok((29, 1)); "29 set")]
#[test_case(false, 0, 0, 32 => Ok((0, 1)); "zero set")]
#[test_case(true, 0, None, 32 => Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext)); "Can't read state from external context")]
#[test_case(false, 1, 29, 32 => Ok((0, 0)); "Wrong contract id")]
#[test_case(false, 0, 29, 33 => Ok((0, 0)); "Wrong key")]
#[test_case(true, 0, None, Word::MAX => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Overflowing key")]
#[test_case(true, 0, None, VM_MAX_RAM => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Overflowing key ram")]
fn test_state_read_word(
    external: bool,
    fp: Word,
    insert: impl Into<Option<Word>>,
    key: Word,
) -> Result<(Word, Word), RuntimeError<Infallible>> {
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[0..ContractId::LEN].copy_from_slice(&[3u8; ContractId::LEN][..]);
    memory[32..64].copy_from_slice(&[4u8; 32][..]);
    let mut pc = 4;
    let mut result = 0;
    let mut got_result = 0;
    let context = if external {
        Context::Script {
            block_height: Default::default(),
        }
    } else {
        Context::Call {
            block_height: Default::default(),
        }
    };

    if let Some(insert) = insert.into() {
        let fp = 0;
        let context = Context::Call {
            block_height: Default::default(),
        };
        let input = StateWordCtx {
            storage: &mut storage,
            memory: &mut memory,
            context: &context,
            fp: Reg::new(&fp),
            pc: RegMut::new(&mut pc),
        };
        state_write_word(input, 32, &mut 0, insert)?;
    }
    let mut pc = 4;

    let input = StateWordCtx {
        storage: &mut storage,
        memory: &mut memory,
        context: &context,
        fp: Reg::new(&fp),
        pc: RegMut::new(&mut pc),
    };
    state_read_word(input, &mut result, &mut got_result, key)?;

    assert_eq!(pc, 8);
    Ok((result, got_result))
}

#[test_case(false, 0, false, 32 => Ok(0); "Nothing set")]
#[test_case(false, 0, true, 32 => Ok(1); "Something set")]
#[test_case(true, 0, false, 32 => Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext)); "Can't write state from external context")]
#[test_case(false, 1, false, 32 => Ok(0); "Wrong contract id")]
#[test_case(false, 0, false, 33 => Ok(0); "Wrong key")]
#[test_case(false, 1, true, 32 => Ok(0); "Wrong contract id with existing")]
#[test_case(false, 0, true, 33 => Ok(0); "Wrong key with existing")]
#[test_case(true, 0, false, Word::MAX => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Overflowing key")]
#[test_case(true, 0, false, VM_MAX_RAM => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Overflowing key ram")]
fn test_state_write_word(
    external: bool,
    fp: Word,
    insert: bool,
    key: Word,
) -> Result<Word, RuntimeError<Infallible>> {
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[0..ContractId::LEN].copy_from_slice(&[3u8; ContractId::LEN][..]);
    memory[32..64].copy_from_slice(&[4u8; 32][..]);
    let mut pc = 4;
    let mut result = 0;
    let context = if external {
        Context::Script {
            block_height: Default::default(),
        }
    } else {
        Context::Call {
            block_height: Default::default(),
        }
    };

    if insert {
        let fp = 0;
        let context = Context::Call {
            block_height: Default::default(),
        };
        let input = StateWordCtx {
            storage: &mut storage,
            memory: &mut memory,
            context: &context,
            fp: Reg::new(&fp),
            pc: RegMut::new(&mut pc),
        };
        state_write_word(input, 32, &mut 0, 20)?;
    }
    let mut pc = 4;

    let input = StateWordCtx {
        storage: &mut storage,
        memory: &mut memory,
        context: &context,
        fp: Reg::new(&fp),
        pc: RegMut::new(&mut pc),
    };
    state_write_word(input, key, &mut result, 30)?;

    assert_eq!(pc, 8);
    Ok(result)
}

#![allow(clippy::cast_possible_truncation)]
use alloc::{
    vec,
    vec::Vec,
};

use core::ops::Range;

use crate::{
    context::Context,
    storage::{
        ContractsStateData,
        MemoryStorage,
        MemoryStorageError,
    },
};
use test_case::test_case;

use super::*;

mod scwq;
mod srwq;
mod swwq;

fn mem(chains: &[&[u8]]) -> MemoryInstance {
    let mut vec: Vec<_> = chains.iter().flat_map(|i| i.iter().copied()).collect();
    vec.resize(MEM_SIZE, 0);
    vec.into()
}
const fn key(k: u8) -> [u8; 32] {
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, k,
    ]
}

fn data(value: &[u8]) -> ContractsStateData {
    ContractsStateData::from(value)
}

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
) -> Result<(Word, Word), RuntimeError<MemoryStorageError>> {
    let mut storage = MemoryStorage::default();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[0..ContractId::LEN].copy_from_slice(&[3u8; ContractId::LEN][..]);
    memory[32..64].copy_from_slice(&[4u8; 32][..]);
    let is = 4;
    let mut cgas = 1000;
    let mut ggas = 1000;
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
        let input = StateWriteWordCtx {
            storage: &mut storage,
            memory: &mut memory,
            context: &context,
            profiler: &mut Profiler::default(),
            new_storage_gas_per_byte: 1,
            current_contract: None,
            cgas: RegMut::new(&mut cgas),
            ggas: RegMut::new(&mut ggas),
            is: Reg::new(&is),
            fp: Reg::new(&fp),
            pc: RegMut::new(&mut pc),
        };
        state_write_word(input, 32, &mut 0, insert)?;
    }
    let mut pc = 4;

    let input = StateReadWordCtx {
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

#[test_case(false, 0, false, 32 => Ok(1); "Nothing set")]
#[test_case(false, 0, true, 32 => Ok(0); "Something set")]
#[test_case(true, 0, false, 32 => Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext)); "Can't write state from external context")]
#[test_case(false, 1, false, 32 => Ok(1); "Wrong contract id")]
#[test_case(false, 0, false, 33 => Ok(1); "Wrong key")]
#[test_case(false, 1, true, 32 => Ok(1); "Wrong contract id with existing")]
#[test_case(false, 0, true, 33 => Ok(1); "Wrong key with existing")]
#[test_case(true, 0, false, Word::MAX => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Overflowing key")]
#[test_case(true, 0, false, VM_MAX_RAM => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Overflowing key ram")]
fn test_state_write_word(
    external: bool,
    fp: Word,
    insert: bool,
    key: Word,
) -> Result<Word, RuntimeError<MemoryStorageError>> {
    let mut storage = MemoryStorage::default();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
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

    let is = 4;
    let mut cgas = 1000;
    let mut ggas = 1000;

    if insert {
        let fp = 0;
        let context = Context::Call {
            block_height: Default::default(),
        };
        let input = StateWriteWordCtx {
            storage: &mut storage,
            memory: &mut memory,
            context: &context,
            profiler: &mut Profiler::default(),
            new_storage_gas_per_byte: 1,
            current_contract: None,
            cgas: RegMut::new(&mut cgas),
            ggas: RegMut::new(&mut ggas),
            is: Reg::new(&is),
            fp: Reg::new(&fp),
            pc: RegMut::new(&mut pc),
        };
        state_write_word(input, 32, &mut 0, 20)?;
    }
    let mut pc = 4;

    let input = StateWriteWordCtx {
        storage: &mut storage,
        memory: &mut memory,
        context: &context,
        new_storage_gas_per_byte: 1,
        current_contract: None,
        profiler: &mut Profiler::default(),
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        is: Reg::new(&is),
        fp: Reg::new(&fp),
        pc: RegMut::new(&mut pc),
    };
    state_write_word(input, key, &mut result, 30)?;

    assert_eq!(pc, 8);
    Ok(result)
}

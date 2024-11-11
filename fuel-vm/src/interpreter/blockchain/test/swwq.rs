#![allow(clippy::type_complexity)]

use alloc::{
    vec,
    vec::Vec,
};

use crate::storage::{
    ContractsState,
    ContractsStateData,
    MemoryStorage,
    MemoryStorageError,
};

use super::*;
use fuel_storage::StorageAsMut;
use test_case::test_case;

struct SWWQInput {
    input: StateWriteQWord,
    storage_slots: Vec<([u8; 32], ContractsStateData)>,
    memory: MemoryInstance,
}

#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: u64::MAX,
            source_pointer: 0,
            num_slots: 1,
        },
        storage_slots: vec![],
        memory: mem(&[]),
    } => matches Err(_)
    ; "Fail when rA + 32 overflows"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 0,
            source_pointer: u64::MAX,
            num_slots: 1,
        },
        storage_slots: vec![],
        memory: mem(&[]),
    } => matches Err(_)
    ; "Fail when rC + 32 * d overflows (rC too high)"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 0,
            source_pointer: 0,
            num_slots: u64::MAX,
        },
        storage_slots: vec![],
        memory: mem(&[]),
    } => matches Err(_)
    ; "Fail when rC + 32 * d overflows (rD too high)"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: VM_MAX_RAM - 1,
            source_pointer: 0,
            num_slots: 1,
        },
        storage_slots: vec![],
        memory: mem(&[]),
    } => matches Err(_)
    ; "Fail when rA + 32 > VM_MAX_RAM"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 0,
            source_pointer: VM_MAX_RAM - 1,
            num_slots: 1,
        },
        storage_slots: vec![],
        memory: mem(&[]),
    } => matches Err(_)
    ; "Fail when rC + 32 * rD > VM_MAX_RAM"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 0,
            source_pointer: VM_MAX_RAM - 63,
            num_slots: 2,
        },
        storage_slots: vec![],
        memory: mem(&[]),
    } => matches Err(_)
    ; "Fail when rC + 32 * rD == VM_MAX_RAM (rD too high)"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: VM_MAX_RAM - 32,
            source_pointer: 0,
            num_slots: 1,
        },
        storage_slots: vec![],
        memory: mem(&[&[0; MEM_SIZE]]),
    } => matches Ok(_)
    ; "Pass when rA + 32 == VM_MAX_RAM"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 2,
            source_pointer: 34,
            num_slots: 1,
        },
        storage_slots: vec![],
        memory: mem(&[&[0; 2], &key(27), &[5; 32]]),
    } => Ok((vec![(key(27), data(&[5; 32]))], 1))
    ; "Single slot write w/ offset key in memory"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 0,
            source_pointer: 32,
            num_slots: 2,
        },
        storage_slots: vec![],
        memory: mem(&[&key(27), &[5; 32], &[6; 32]]),
    } => Ok((vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32]))], 2))
    ; "Two slot write"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 0,
            source_pointer: 32,
            num_slots: 2,
        },
        storage_slots: vec![(key(27), data(&[2; 32]))],
        memory: mem(&[&key(27), &[5; 32], &[6; 32]]),
    } => Ok((vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32]))], 1))
    ; "Two slot writes with one pre-existing slot set"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 0,
            source_pointer: 32,
            num_slots: 2,
        },
        storage_slots: vec![],
        memory: mem(&[&key(27), &[5; 32], &[6; 32], &[7; 32]]),
    } => Ok((vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32]))], 2))
    ; "Only writes two slots when memory has more data available"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 0,
            source_pointer: 32,
            num_slots: 3,
        },
        storage_slots: vec![],
        memory: mem(&[&key(27), &[5; 32], &[6; 32], &[7; 32]]),
    } => Ok((vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32])), (key(29), data(&[7; 32]))], 3))
    ; "Three slot write"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 0,
            source_pointer: 32,
            num_slots: 3,
        },
        storage_slots: vec![(key(29), data(&[8; 32]))],
        memory: mem(&[&key(27), &[5; 32], &[6; 32], &[7; 32]]),
    } => Ok((vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32])), (key(29), data(&[7; 32]))], 2))
    ; "Three slot write with one pre-existing slot set"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 0,
            source_pointer: 32,
            num_slots: 3,
        },
        storage_slots: vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32])), (key(29), data(&[7; 32]))],
        memory: mem(&[&key(27), &[5; 32], &[6; 32], &[7; 32]]),
    } => Ok((vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32])), (key(29), data(&[7; 32]))], 0))
    ; "Three slot write with all slots previously set"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 0,
            source_pointer: 32,
            num_slots: 2,
        },
        storage_slots: vec![(key(29), data(&[8; 32]))],
        memory: mem(&[&key(27), &[5; 32], &[6; 32], &[7; 32]]),
    } => Ok((vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32])), (key(29), data(&[8; 32]))], 2))
    ; "Does not override slots that aren't being written to (adjacent)"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord {
            starting_storage_key_pointer: 0,
            source_pointer: 32,
            num_slots: 3,
        },
        storage_slots: vec![(key(100), data(&[8; 32]))],
        memory: mem(&[&key(27), &[5; 32], &[6; 32], &[7; 32]]),
    } => Ok((vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32])), (key(29), data(&[7; 32])), (key(100), data(&[8; 32]))], 3))
    ; "Does not override slots that aren't being written to (non-adjacent)"
)]
fn test_state_write_qword(
    input: SWWQInput,
) -> Result<(Vec<([u8; 32], ContractsStateData)>, u64), RuntimeError<MemoryStorageError>>
{
    let SWWQInput {
        input,
        storage_slots,
        memory,
    } = input;
    let mut storage = MemoryStorage::default();

    for (k, v) in storage_slots {
        storage
            .storage::<ContractsState>()
            .insert(
                &(&ContractId::default(), &Bytes32::new(k)).into(),
                v.as_ref(),
            )
            .unwrap();
    }

    let mut result_register = 0u64;
    let is = 0;
    let mut cgas = 10_000;
    let mut ggas = 10_000;
    let mut pc = 0;
    state_write_qword(
        &Default::default(),
        &mut storage,
        &memory,
        &mut Profiler::default(),
        1,
        None,
        RegMut::new(&mut cgas),
        RegMut::new(&mut ggas),
        Reg::new(&is),
        RegMut::new(&mut pc),
        &mut result_register,
        input,
    )?;

    let results = storage
        .all_contract_state()
        .map(|(key, v)| (**key.state_key(), v.clone()))
        .collect();
    Ok((results, result_register))
}

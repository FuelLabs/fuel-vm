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

struct SCWQInput {
    input: StateClearQWord,
    storage_slots: Vec<([u8; 32], ContractsStateData)>,
    memory: MemoryInstance,
}

#[test_case(
    SCWQInput{
        input: StateClearQWord::new(u64::MAX, 1).unwrap(),
        storage_slots: vec![(key(27), data(&[8; 32]))],
        memory: mem(&[&key(27)]),
    } => matches Err(_)
    ; "Fail when $rA + 32 overflows"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(u64::MAX - 1, 1).unwrap(),
        storage_slots: vec![(key(27), data(&[8; 32]))],
        memory: mem(&[&key(27)]),
    } => matches Err(_)
    ; "Fail when $rA + 32 > VM_MAX_RAM"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(VM_MAX_RAM - 32, 1).unwrap(),
        storage_slots: vec![(key(27), data(&[8; 32]))],
        memory: mem(&[&key(27)]),
    } => matches Ok(_)
    ; "Pass when $rA + 32 == VM_MAX_RAM"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(0, 1).unwrap(),
        storage_slots: vec![(key(27), data(&[8; 32]))],
        memory: mem(&[&key(27)]),
    } => Ok((vec![], true))
    ; "Clear single storage slot"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(0, 2).unwrap(),
        storage_slots: vec![(key(27), data(&[8; 32])), (key(28), data(&[9; 32]))],
        memory: mem(&[&key(27)]),
    } => Ok((vec![], true))
    ; "Clear multiple existing storage slots"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(0, 1).unwrap(),
        storage_slots: vec![],
        memory: mem(&[&key(27)]),
    } => Ok((vec![], false))
    ; "Clear single storage slot that was never set"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(0, 2000).unwrap(),
        storage_slots: vec![],
        memory: mem(&[&key(27)]),
    } => Ok((vec![], false))
    ; "Clear u64::MAX storage slot that was never set"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(0, 2).unwrap(),
        storage_slots: vec![(key(27), data(&[8; 32])), (key(29), data(&[8; 32]))],
        memory: mem(&[&key(27)]),
    } => Ok((vec![(key(29), vec![8; 32].into())], false))
    ; "Clear storage slots with some previously set"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(0, 2).unwrap(),
        storage_slots: vec![(key(27), data(&[8; 32])), (key(26), data(&[8; 32]))],
        memory: mem(&[&key(27)]),
    } => Ok((vec![(key(26), vec![8; 32].into())], false))
    ; "Clear storage slots with some previously set before the key"
)]
fn test_state_clear_qword(
    input: SCWQInput,
) -> Result<(Vec<([u8; 32], ContractsStateData)>, bool), RuntimeError<MemoryStorageError>>
{
    let SCWQInput {
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
    let mut pc = 0;
    state_clear_qword(
        &Default::default(),
        &mut storage,
        &memory,
        RegMut::new(&mut pc),
        &mut result_register,
        input,
    )?;

    let results = storage
        .all_contract_state()
        .map(|(key, v)| (**key.state_key(), v.clone()))
        .collect();
    Ok((results, result_register != 0))
}

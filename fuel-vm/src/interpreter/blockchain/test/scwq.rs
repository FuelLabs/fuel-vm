#![allow(clippy::type_complexity)]

use alloc::{
    vec,
    vec::Vec,
};

use crate::storage::{
    ContractsState,
    MemoryStorage,
};

use super::*;
use fuel_storage::StorageAsMut;
use test_case::test_case;

struct SCWQInput {
    input: StateClearQWord,
    storage_slots: Vec<([u8; 32], [u8; 32])>,
    memory: Memory,
}

#[test_case(
    SCWQInput{
        input: StateClearQWord::new(0, 1).unwrap(),
        storage_slots: vec![(key(27), [8; 32])],
        memory: mem(&[&key(27)]),
    } => (vec![], true)
    ; "Clear single storage slot"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(0, 2).unwrap(),
        storage_slots: vec![(key(27), [8; 32]), (key(28), [9; 32])],
        memory: mem(&[&key(27)]),
    } => (vec![], true)
    ; "Clear multiple existing storage slots"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(0, 1).unwrap(),
        storage_slots: vec![],
        memory: mem(&[&key(27)]),
    } => (vec![], false)
    ; "Clear single storage slot that was never set"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(0, 2000).unwrap(),
        storage_slots: vec![],
        memory: mem(&[&key(27)]),
    } => (vec![], false)
    ; "Clear u64::MAX storage slot that was never set"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(0, 2).unwrap(),
        storage_slots: vec![(key(27), [8; 32]), (key(29), [8; 32])],
        memory: mem(&[&key(27)]),
    } => (vec![(key(29), [8; 32])], false)
    ; "Clear storage slots with some previously set"
)]
#[test_case(
    SCWQInput{
        input: StateClearQWord::new(0, 2).unwrap(),
        storage_slots: vec![(key(27), [8; 32]), (key(26), [8; 32])],
        memory: mem(&[&key(27)]),
    } => (vec![(key(26), [8; 32])], false)
    ; "Clear storage slots with some previously set before the key"
)]
fn test_state_clear_qword(input: SCWQInput) -> (Vec<([u8; 32], [u8; 32])>, bool) {
    let SCWQInput {
        input,
        storage_slots,
        memory,
    } = input;
    let mut storage = MemoryStorage::new(Default::default(), Default::default());

    for (k, v) in storage_slots {
        storage
            .storage::<ContractsState>()
            .insert(
                &(&ContractId::default(), &Bytes32::new(k)).into(),
                &Bytes32::new(v),
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
    )
    .unwrap();

    let results = storage
        .all_contract_state()
        .map(|(key, v)| (**key.state_key(), **v))
        .collect();
    (results, result_register != 0)
}

#[test_case(
    0, 1
    => matches Ok(_)
    ; "Pass when values are in a valid range"
)]
#[test_case(
    u64::MAX, 1
    => matches Err(_)
    ; "Fail when $rA + 32 overflows"
)]
#[test_case(
    VM_MAX_RAM-1, 1
    => matches Err(_)
    ; "Fail when $rA + 32 > VM_MAX_RAM"
)]
#[test_case(
    VM_MAX_RAM-32, 1
    => matches Ok(_)
    ; "Pass when $rA + 32 == VM_MAX_RAM"
)]
fn test_state_clear_qword_input(
    start_key_memory_address: Word,
    num_slots: Word,
) -> SimpleResult<()> {
    StateClearQWord::new(start_key_memory_address, num_slots).map(|_| ())
}

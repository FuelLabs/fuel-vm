use crate::context::Context;
use crate::storage::ContractsState;
use crate::storage::MemoryStorage;

use super::*;
use fuel_storage::StorageAsMut;
use test_case::test_case;

struct SCWQInput {
    input: StateClearQWord,
    storage_slots: Vec<([u8; 32], [u8; 32])>,
    memory: Vec<u8>,
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
    let mut storage = MemoryStorage::new(0, Default::default());

    for (k, v) in storage_slots {
        storage
            .storage::<ContractsState>()
            .insert(&(&ContractId::default(), &Bytes32::new(k)), &Bytes32::new(v))
            .unwrap();
    }

    let mut result_register = 0u64;
    state_clear_qword(&Default::default(), &mut storage, &memory, &mut result_register, input).unwrap();

    let results = storage.all_contract_state().map(|((_, k), v)| (**k, **v)).collect();
    (results, result_register != 0)
}

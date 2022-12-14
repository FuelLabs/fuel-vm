use crate::context::Context;
use crate::storage::ContractsState;
use crate::storage::MemoryStorage;

use super::*;
use fuel_storage::StorageAsMut;
use test_case::test_case;

struct SWWQInput {
    input: StateWriteQWord,
    storage_slots: Vec<([u8; 32], [u8; 32])>,
    memory: Vec<u8>,
}

#[test_case(
    SWWQInput{
        input: StateWriteQWord::new(2, 34, 1).unwrap(),
        storage_slots: vec![],
        memory: mem(&[&[0; 2], &key(27), &[5; 32]]),
    } => (vec![(key(27), [5; 32])], false)
    ; "Single slot write w/ offset key in memory"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord::new(0, 32, 2).unwrap(),
        storage_slots: vec![],
        memory: mem(&[&key(27), &[5; 32], &[6; 32]]),
    } => (vec![(key(27), [5; 32]), (key(28), [6; 32])], false)
    ; "Two slot write"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord::new(0, 32, 2).unwrap(),
        storage_slots: vec![(key(27), [2; 32])],
        memory: mem(&[&key(27), &[5; 32], &[6; 32]]),
    } => (vec![(key(27), [5; 32]), (key(28), [6; 32])], false)
    ; "Two slot writes with one pre-existing slot set"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord::new(0, 32, 2).unwrap(),
        storage_slots: vec![],
        memory: mem(&[&key(27), &[5; 32], &[6; 32], &[7; 32]]),
    } => (vec![(key(27), [5; 32]), (key(28), [6; 32])], false)
    ; "Only writes two slots when memory has more data available"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord::new(0, 32, 3).unwrap(),
        storage_slots: vec![],
        memory: mem(&[&key(27), &[5; 32], &[6; 32], &[7; 32]]),
    } => (vec![(key(27), [5; 32]), (key(28), [6; 32]), (key(29), [7; 32])], false)
    ; "Three slot write"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord::new(0, 32, 3).unwrap(),
        storage_slots: vec![(key(29), [8; 32])],
        memory: mem(&[&key(27), &[5; 32], &[6; 32], &[7; 32]]),
    } => (vec![(key(27), [5; 32]), (key(28), [6; 32]), (key(29), [7; 32])], false)
    ; "Three slot write with one pre-existing slot set"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord::new(0, 32, 3).unwrap(),
        storage_slots: vec![(key(27), [5; 32]), (key(28), [6; 32]), (key(29), [7; 32])],
        memory: mem(&[&key(27), &[5; 32], &[6; 32], &[7; 32]]),
    } => (vec![(key(27), [5; 32]), (key(28), [6; 32]), (key(29), [7; 32])], true)
    ; "Three slot write with all slots previously set"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord::new(0, 32, 2).unwrap(),
        storage_slots: vec![(key(29), [8; 32])],
        memory: mem(&[&key(27), &[5; 32], &[6; 32], &[7; 32]]),
    } => (vec![(key(27), [5; 32]), (key(28), [6; 32]), (key(29), [8; 32])], false)
    ; "Does not override slots that aren't being written to (adjacent)"
)]
#[test_case(
    SWWQInput{
        input: StateWriteQWord::new(0, 32, 3).unwrap(),
        storage_slots: vec![(key(100), [8; 32])],
        memory: mem(&[&key(27), &[5; 32], &[6; 32], &[7; 32]]),
    } => (vec![(key(27), [5; 32]), (key(28), [6; 32]), (key(29), [7; 32]), (key(100), [8; 32])], false)
    ; "Does not override slots that aren't being written to (non-adjacent)"
)]
fn test_state_write_qword(input: SWWQInput) -> (Vec<([u8; 32], [u8; 32])>, bool) {
    let SWWQInput {
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
    state_write_qword(&Default::default(), &mut storage, &memory, &mut result_register, input).unwrap();

    let results = storage.all_contract_state().map(|((_, k), v)| (**k, **v)).collect();
    (results, result_register != 0)
}

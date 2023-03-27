#![allow(clippy::type_complexity)]

use crate::storage::ContractsState;
use crate::storage::MemoryStorage;

use super::*;
use fuel_storage::StorageAsMut;
use test_case::test_case;

struct SWWQInput {
    input: StateWriteQWord,
    storage_slots: Vec<([u8; 32], [u8; 32])>,
    memory: Memory<MEM_SIZE>,
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
    let mut storage = MemoryStorage::new(Default::default(), Default::default());

    for (k, v) in storage_slots {
        storage
            .storage::<ContractsState>()
            .insert(&(&ContractId::default(), &Bytes32::new(k)).into(), &Bytes32::new(v))
            .unwrap();
    }

    let mut result_register = 0u64;
    let mut pc = 0;
    state_write_qword(
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
    0, 0, 1
    => matches Ok(_)
    ; "Pass when values are within valid ranges"
)]
#[test_case(
    u64::MAX, 0, 1
    => matches Err(_)
    ; "Fail when rA + 32 overflows"
)]
#[test_case(
    0, u64::MAX, 1
    => matches Err(_)
    ; "Fail when rC + 32 * d overflows (rC too high)"
)]
#[test_case(
    0, 0, u64::MAX
    => matches Err(_)
    ; "Fail when rC + 32 * d overflows (rD too high)"
)]
#[test_case(
    VM_MAX_RAM - 1, 0, 1
    => matches Err(_)
    ; "Fail when rA + 32 > VM_MAX_RAM"
)]
#[test_case(
    VM_MAX_RAM - 32, 0, 1
    => matches Ok(_)
    ; "Pass when rA + 32 == VM_MAX_RAM"
)]
#[test_case(
    0, VM_MAX_RAM - 1, 1
    => matches Err(_)
    ; "Fail when rC + 32 * rD > VM_MAX_RAM"
)]
#[test_case(
    0, VM_MAX_RAM - 32, 1
    => matches Ok(_)
    ; "Pass when rC + 32 * rD == VM_MAX_RAM"
)]
#[test_case(
    0, VM_MAX_RAM - 63, 2
    => matches Err(_)
    ; "Fail when rC + 32 * rD == VM_MAX_RAM (rD too high)"
)]
fn test_state_write_qword_input(
    start_key_memory_address: Word,
    source_memory_address: Word,
    num_slots: Word,
) -> Result<(), RuntimeError> {
    StateWriteQWord::new(start_key_memory_address, source_memory_address, num_slots).map(|_| ())
}

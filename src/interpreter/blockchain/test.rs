use crate::storage::ContractsState;
use crate::storage::MemoryStorage;

use super::*;
use fuel_storage::StorageAsMut;
use test_case::test_case;

struct SRWQInput {
    input: StateReadQWord,
    storage_slots: Vec<([u8; 32], [u8; 32])>,
    memory: Vec<u8>,
}

fn mem(chains: &[&[u8]]) -> Vec<u8> {
    let mut vec: Vec<_> = chains.iter().flat_map(|i| i.iter().copied()).collect();
    vec.resize(200, 0);
    vec
}
const fn key(k: u8) -> [u8; 32] {
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, k,
    ]
}

#[test_case(
    SRWQInput{
        input: StateReadQWord::new(34, 2, 1).unwrap(),
        storage_slots: vec![(key(27), [5; 32])],
        memory: mem(&[&[0; 2], &key(27), &1u64.to_be_bytes()]),
    } => (mem(&[&[0], &[5; 32], &[27], &1u64.to_be_bytes()]), false)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::new(32, 0, 2).unwrap(),
        storage_slots: vec![(key(27), [5; 32]), (key(28), [6; 32])],
        memory: mem(&[&key(27), &0u64.to_be_bytes()]),
    } => (mem(&[&[5; 32], &[6; 32]]), false)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::new(32, 0, 3).unwrap(),
        storage_slots: vec![(key(27), [5; 32]), (key(28), [6; 32]), (key(29), [7; 32])],
        memory: mem(&[&key(27), &0u64.to_be_bytes()]),
    } => (mem(&[&[5; 32], &[6; 32], &[7; 32]]), false)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::new(32, 0, 2).unwrap(),
        storage_slots: vec![(key(27), [5; 32]), (key(28), [6; 32]), (key(29), [7; 32])],
        memory: mem(&[&key(27), &0u64.to_be_bytes()]),
    } => (mem(&[&[5; 32], &[6; 32]]), false)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::new(32, 0, 3).unwrap(),
        storage_slots: vec![(key(27), [5; 32]), (key(30), [6; 32]), (key(29), [7; 32])],
        memory: mem(&[&key(27), &0u64.to_be_bytes()]),
    } => (mem(&[&[5; 32], &[0; 32], &[7; 32]]), true)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::new(32, 0, 3).unwrap(),
        storage_slots: vec![(key(27), [5; 32]), (key(28), [7; 32])],
        memory: mem(&[&key(27), &0u64.to_be_bytes()]),
    } => (mem(&[&[5; 32], &[7; 32], &[0; 32]]), true)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::new(32, 0, 3).unwrap(),
        storage_slots: vec![(key(26), [5; 32]), (key(28), [6; 32]), (key(29), [7; 32])],
        memory: mem(&[&key(27), &0u64.to_be_bytes()]),
    } => (mem(&[&[0; 32], &[6; 32], &[7; 32]]), true)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::new(32, 0, 3).unwrap(),
        storage_slots: vec![(key(28), [6; 32]), (key(29), [7; 32])],
        memory: mem(&[&key(27), &0u64.to_be_bytes()]),
    } => (mem(&[&[0; 32], &[6; 32], &[7; 32]]), true)
)]
fn test_state_read_qword(input: SRWQInput) -> (Vec<u8>, bool) {
    let SRWQInput {
        input,
        storage_slots,
        mut memory,
    } = input;
    let mut storage = MemoryStorage::new(0, Default::default());
    for (k, v) in storage_slots {
        storage
            .storage::<ContractsState>()
            .insert(&(&ContractId::default(), &Bytes32::new(k)), &Bytes32::new(v))
            .unwrap();
    }
    let mut result_register = 0u64;
    state_read_qword(
        &Default::default(),
        &mut storage,
        &mut memory,
        &mut result_register,
        input,
    )
    .unwrap();
    (memory, result_register != 0)
}

struct SWWQInput {
    input: StateWriteQWord,
    storage_slots: Vec<([u8; 32], [u8; 32])>,
    memory: Vec<u8>,
}

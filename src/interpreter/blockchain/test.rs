use crate::context::Context;
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
impl StateReadQWord {
    fn test(
        destination_memory_address: Word,
        origin_key_memory_address: Word,
        num_slots: Word,
    ) -> Result<Self, RuntimeError> {
        let r = OwnershipRegisters {
            sp: u64::MAX / 2,
            ssp: 0,
            hp: u64::MAX / 2 + 1,
            prev_hp: u64::MAX,
            context: crate::context::Context::Call { block_height: 0 },
        };
        Self::new(destination_memory_address, origin_key_memory_address, num_slots, r)
    }
}

impl OwnershipRegisters {
    fn test(stack: Range<u64>, heap: Range<u64>, context: Context) -> Self {
        Self {
            sp: stack.end,
            ssp: stack.start,
            hp: heap.start,
            prev_hp: heap.end,
            context,
        }
    }
}
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(1, 2, 1).unwrap(),
        storage_slots: vec![(key(27), [5; 32])],
        memory: mem(&[&[0; 2], &key(27)]),
    } => (mem(&[&[0], &[5; 32], &[27]]), false)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 2).unwrap(),
        storage_slots: vec![(key(27), [5; 32]), (key(28), [6; 32])],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[5; 32], &[6; 32]]), false)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 3).unwrap(),
        storage_slots: vec![(key(27), [5; 32]), (key(28), [6; 32]), (key(29), [7; 32])],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[5; 32], &[6; 32], &[7; 32]]), false)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 2).unwrap(),
        storage_slots: vec![(key(27), [5; 32]), (key(28), [6; 32]), (key(29), [7; 32])],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[5; 32], &[6; 32]]), false)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 3).unwrap(),
        storage_slots: vec![(key(27), [5; 32]), (key(30), [6; 32]), (key(29), [7; 32])],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[5; 32], &[0; 32], &[7; 32]]), true)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 3).unwrap(),
        storage_slots: vec![(key(27), [5; 32]), (key(28), [7; 32])],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[5; 32], &[7; 32], &[0; 32]]), true)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 3).unwrap(),
        storage_slots: vec![(key(26), [5; 32]), (key(28), [6; 32]), (key(29), [7; 32])],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[0; 32], &[6; 32], &[7; 32]]), true)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 3).unwrap(),
        storage_slots: vec![(key(28), [6; 32]), (key(29), [7; 32])],
        memory: mem(&[&key(27)]),
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

#[test_case(
    0, 0, 1, OwnershipRegisters::test(0..100, 100..200, Context::Call{block_height: 0})
    => matches Ok(())
    ; "Ownership check passes when destination is within allocated stack"
)]
#[test_case(
    2, 0, 1, OwnershipRegisters::test(0..1, 3..4, Context::Call{block_height: 0})
    => matches Err(_)
    ; "Ownership check fails when destination is in-between stack and heap"
)]
#[test_case(
    2, 0, 1, OwnershipRegisters::test(0..4, 5..6, Context::Call{block_height: 0})
    => matches Err(_)
    ; "Ownership check fails when stack is too small"
)]
#[test_case(
    3, 0, 1, OwnershipRegisters::test(0..1, 2..35, Context::Call{block_height: 0})
    => matches Ok(_)
    ; "Ownership check passes when heap is large enough"
)]
#[test_case(
    VM_MAX_RAM, 0, 1, OwnershipRegisters::test(0..1, 3..u64::MAX, Context::Call{block_height: 0})
    => matches Err(_)
    ; "Ownership check fails "
)]
#[test_case(
    3, 0, 1, OwnershipRegisters::test(0..1, 2..4, Context::Call{block_height: 0})
    => matches Err(_)
)]
fn test_state_read_qword_input(
    destination_memory_address: Word,
    origin_key_memory_address: Word,
    num_slots: Word,
    ownership_registers: OwnershipRegisters,
) -> Result<(), RuntimeError> {
    StateReadQWord::new(
        destination_memory_address,
        origin_key_memory_address,
        num_slots,
        ownership_registers,
    )
    .map(|_| ())
}

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

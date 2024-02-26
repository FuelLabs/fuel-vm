use crate::{
    context::Context,
    storage::{
        ContractsState,
        MemoryStorage,
    },
};

use super::*;
use fuel_storage::StorageAsMut;
use fuel_tx::ContractsStateData;
use test_case::test_case;

struct SRWQInput {
    input: StateReadQWord,
    storage_slots: Vec<([u8; 32], ContractsStateData)>,
    memory: Memory<MEM_SIZE>,
}

impl StateReadQWord {
    fn test(
        destination_memory_address: Word,
        origin_key_memory_address: Word,
        num_slots: Word,
    ) -> SimpleResult<Self> {
        let r = OwnershipRegisters {
            sp: u64::MAX / 2,
            ssp: 0,
            hp: u64::MAX / 2 + 1,
            prev_hp: u64::MAX,
            context: crate::context::Context::Call {
                block_height: Default::default(),
            },
        };
        Self::new(
            destination_memory_address,
            origin_key_memory_address,
            num_slots,
            r,
        )
    }
}

#[test_case(
    SRWQInput{
        input: StateReadQWord::test(1, 2, 1).unwrap(),
        storage_slots: vec![(key(27), data(&[5; 32]))],
        memory: mem(&[&[0; 2], &key(27)]),
    } => (mem(&[&[0], &[5; 32], &[27]]), true)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 2).unwrap(),
        storage_slots: vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32]))],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[5; 32], &[6; 32]]), true)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 3).unwrap(),
        storage_slots: vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32])), (key(29), data(&[7; 32]))],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[5; 32], &[6; 32], &[7; 32]]), true)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 2).unwrap(),
        storage_slots: vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32])), (key(29), data(&[7; 32]))],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[5; 32], &[6; 32]]), true)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 3).unwrap(),
        storage_slots: vec![(key(27), data(&[5; 32])), (key(30), data(&[6; 32])), (key(29), data(&[7; 32]))],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[5; 32], &[0; 32], &[7; 32]]), false)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 3).unwrap(),
        storage_slots: vec![(key(27), data(&[5; 32])), (key(28), data(&[7; 32]))],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[5; 32], &[7; 32], &[0; 32]]), false)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 3).unwrap(),
        storage_slots: vec![(key(26), data(&[5; 32])), (key(28), data(&[6; 32])), (key(29), data(&[7; 32]))],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[0; 32], &[6; 32], &[7; 32]]), false)
)]
#[test_case(
    SRWQInput{
        input: StateReadQWord::test(0, 0, 3).unwrap(),
        storage_slots: vec![(key(28), data(&[6; 32])), (key(29), data(&[7; 32]))],
        memory: mem(&[&key(27)]),
    } => (mem(&[&[0; 32], &[6; 32], &[7; 32]]), false)
)]
fn test_state_read_qword(input: SRWQInput) -> (Memory<MEM_SIZE>, bool) {
    let SRWQInput {
        input,
        storage_slots,
        mut memory,
    } = input;
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
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
    state_read_qword(
        &Default::default(),
        &storage,
        &mut memory,
        RegMut::new(&mut pc),
        &mut result_register,
        input,
    )
    .unwrap();
    (memory, result_register != 0)
}

#[test_case(
    0, 0, 1, OwnershipRegisters::test(0..100, 100..200, Context::Call{ block_height: Default::default()})
    => matches Ok(())
    ; "Ownership check passes when destination is within allocated stack"
)]
#[test_case(
    2, 0, 1, OwnershipRegisters::test(0..1, 3..4, Context::Call{ block_height: Default::default()})
    => matches Err(_)
    ; "Ownership check fails when destination is in-between stack and heap"
)]
#[test_case(
    2, 0, 1, OwnershipRegisters::test(0..4, 5..6, Context::Call {block_height: Default::default()})
    => matches Err(_)
    ; "Ownership check fails when stack is too small"
)]
#[test_case(
    3, 0, 1, OwnershipRegisters::test(0..1, 2..35, Context::Call{ block_height: Default::default()})
    => matches Ok(_)
    ; "Ownership check passes when heap is large enough"
)]
#[test_case(
    VM_MAX_RAM, 0, 1, OwnershipRegisters::test(0..1, 3..u64::MAX, Context::Call{ block_height: Default::default()})
    => matches Err(_)
    ; "Ownership check fails when destination range exceeds VM MAX"
)]
#[test_case(
    4, VM_MAX_RAM, 1, OwnershipRegisters::test(0..1, 3..u64::MAX, Context::Call{ block_height: Default::default()})
    => matches Err(_)
    ; "Fail when start key memory range exceeds VM_MAX_RAM"
)]
#[test_case(
    4, u64::MAX, 1, OwnershipRegisters::test(0..1, 3..u64::MAX, Context::Call{ block_height: Default::default()})
    => matches Err(_)
    ; "Fail when start key memory range exceeds u64::MAX"
)]
#[test_case(
    4, 0, 1, OwnershipRegisters::test(0..1, 3..u64::MAX, Context::Script { block_height: Default::default()})
    => matches Err(_)
    ; "Fail when context is not inside of a call"
)]
fn test_state_read_qword_input(
    destination_memory_address: Word,
    origin_key_memory_address: Word,
    num_slots: Word,
    ownership_registers: OwnershipRegisters,
) -> SimpleResult<()> {
    StateReadQWord::new(
        destination_memory_address,
        origin_key_memory_address,
        num_slots,
        ownership_registers,
    )
    .map(|_| ())
}

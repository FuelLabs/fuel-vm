use crate::storage::{
    ContractsState,
    ContractsStateData,
    MemoryStorage,
};

use super::*;
use fuel_storage::StorageAsMut;
use test_case::test_case;

const DEFAULT_OWNER: OwnershipRegisters = OwnershipRegisters {
    sp: u64::MAX / 2,
    ssp: 0,
    hp: u64::MAX / 2 + 1,
    prev_hp: u64::MAX,
    context: crate::context::Context::Call {
        block_height: BlockHeight::new(0),
    },
};

struct SRWQInput {
    storage_slots: Vec<([u8; 32], ContractsStateData)>,
    memory: Memory,
    owner: OwnershipRegisters,
    destination_pointer: Word,
    origin_key_pointer: Word,
    num_slots: Word,
}

#[test_case(
    SRWQInput{
        storage_slots: vec![],
        memory: mem(&[]),
        owner: OwnershipRegisters::test(0..1, 3..u64::MAX, Context::Call {
block_height: Default::default()}),
        destination_pointer: VM_MAX_RAM,
        origin_key_pointer: 0,
        num_slots: 1,
    } => matches Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow))
    ; "Fail when destination range exceeds VM MAX"
)]
#[test_case(
    SRWQInput{
        storage_slots: vec![],
        memory: mem(&[]),
        owner: OwnershipRegisters::test(0..1, 3..u64::MAX, Context::Call {
block_height: Default::default()}),
        destination_pointer: 4,
        origin_key_pointer: VM_MAX_RAM,
        num_slots: 1,
    } => matches Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow))
    ; "Fail when start key memory range exceeds VM_MAX_RAM"
)]
#[test_case(
    SRWQInput{
        storage_slots: vec![],
        memory: mem(&[]),
        owner: OwnershipRegisters::test(0..1, 3..u64::MAX, Context::Call {
block_height: Default::default()}),
        destination_pointer: 4,
        origin_key_pointer: u64::MAX,
        num_slots: 1,
    } => matches Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow))
    ; "Fail when start key memory range exceeds u64::MAX"
)]
#[test_case(
    SRWQInput{
        storage_slots: vec![],
        memory: mem(&[]),
        owner: OwnershipRegisters::test(0..1, 3..u64::MAX, Context::Script {
block_height: Default::default()}),
        destination_pointer: 4,
        origin_key_pointer: 0,
        num_slots: 1,
    } => matches Err(RuntimeError::Recoverable(PanicReason::ExpectedInternalContext))
    ; "Fail when context is not inside of a call"
)]
#[test_case(
    SRWQInput{
        storage_slots: vec![(key(27), data(&[5; 32]))],
        memory: mem(&[&[0; 2], &key(27)]),
        owner: DEFAULT_OWNER,
        destination_pointer: 1,
        origin_key_pointer: 2,
        num_slots: 1,
    } => Ok((mem(&[&[0], &[5; 32], &[27]]), true))
)]
#[test_case(
    SRWQInput{
        storage_slots: vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32]))],
        memory: mem(&[&key(27)]),
        owner: DEFAULT_OWNER,
        destination_pointer: 0,
        origin_key_pointer: 0,
        num_slots: 2,
    } => Ok((mem(&[&[5; 32], &[6; 32]]), true))
)]
#[test_case(
    SRWQInput{
        storage_slots: vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32])), (key(29), data(&[7; 32]))],
        memory: mem(&[&key(27)]),
        owner: DEFAULT_OWNER,
        destination_pointer: 0,
        origin_key_pointer: 0,
        num_slots: 3,
    } => Ok((mem(&[&[5; 32], &[6; 32], &[7; 32]]), true))
)]
#[test_case(
    SRWQInput{
        storage_slots: vec![(key(27), data(&[5; 32])), (key(28), data(&[6; 32])), (key(29), data(&[7; 32]))],
        memory: mem(&[&key(27)]),
        owner: DEFAULT_OWNER,
        destination_pointer: 0,
        origin_key_pointer: 0,
        num_slots: 2,
    } => Ok((mem(&[&[5; 32], &[6; 32]]), true))
)]
#[test_case(
    SRWQInput{
        storage_slots: vec![(key(27), data(&[5; 32])), (key(30), data(&[6; 32])), (key(29), data(&[7; 32]))],
        memory: mem(&[&key(27)]),
        owner: DEFAULT_OWNER,
        destination_pointer: 0,
        origin_key_pointer: 0,
        num_slots: 3,
    } => Ok((mem(&[&[5; 32], &[0; 32], &[7; 32]]), false))
)]
#[test_case(
    SRWQInput{
        storage_slots: vec![(key(27), data(&[5; 32])), (key(28), data(&[7; 32]))],
        memory: mem(&[&key(27)]),
        owner: DEFAULT_OWNER,
        destination_pointer: 0,
        origin_key_pointer: 0,
        num_slots: 3,
    } => Ok((mem(&[&[5; 32], &[7; 32], &[0; 32]]), false))
)]
#[test_case(
    SRWQInput{
        storage_slots: vec![(key(26), data(&[5; 32])), (key(28), data(&[6; 32])), (key(29), data(&[7; 32]))],
        memory: mem(&[&key(27)]),
        owner: DEFAULT_OWNER,
        destination_pointer: 0,
        origin_key_pointer: 0,
        num_slots: 3,
    } => Ok((mem(&[&[0; 32], &[6; 32], &[7; 32]]), false))
)]
#[test_case(
    SRWQInput{
        storage_slots: vec![(key(28), data(&[6; 32])), (key(29), data(&[7; 32]))],
        memory: mem(&[&key(27)]),
        owner: DEFAULT_OWNER,
        destination_pointer: 0,
        origin_key_pointer: 0,
        num_slots: 3,
    } => Ok((mem(&[&[0; 32], &[6; 32], &[7; 32]]), false))
)]
fn test_state_read_qword(
    input: SRWQInput,
) -> Result<(Memory, bool), RuntimeError<Infallible>> {
    let SRWQInput {
        storage_slots,
        mut memory,
        owner,
        destination_pointer,
        origin_key_pointer,
        num_slots,
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
    state_read_qword(
        &Default::default(),
        &storage,
        &mut memory,
        RegMut::new(&mut pc),
        owner,
        &mut result_register,
        StateReadQWordParams {
            destination_pointer,
            origin_key_pointer,
            num_slots,
        },
    )?;
    Ok((memory, result_register != 0))
}

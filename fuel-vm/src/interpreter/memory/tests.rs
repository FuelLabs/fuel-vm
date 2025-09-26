#![allow(clippy::cast_possible_truncation)]

use alloc::vec;
use core::ops::Range;

use super::*;
use crate::{
    interpreter::InterpreterParams,
    prelude::*,
    storage::ContractsRawCode,
};
use fuel_asm::op;
use fuel_tx::ConsensusParameters;
use test_case::test_case;

#[cfg(feature = "random")]
#[test]
fn memcopy() {
    let tx_params = TxParameters::default().with_max_gas_per_tx(Word::MAX / 2);
    let zero_gas_price = 0;

    let mut consensus_params = ConsensusParameters::default();
    consensus_params.set_tx_params(tx_params);

    let mut vm = Interpreter::<_, _, _>::with_storage(
        MemoryInstance::new(),
        MemoryStorage::default(),
        InterpreterParams::new(zero_gas_price, &consensus_params),
    );
    let tx = TransactionBuilder::script(op::ret(0x10).to_bytes().to_vec(), vec![])
        .script_gas_limit(100_000)
        .add_fee_input()
        .finalize();

    let tx = tx
        .into_checked(Default::default(), &consensus_params)
        .expect("default tx should produce a valid checked transaction")
        .into_ready(
            zero_gas_price,
            consensus_params.gas_costs(),
            consensus_params.fee_params(),
            None,
        )
        .unwrap();

    vm.init_script(tx).expect("Failed to init VM");

    let alloc = 1024;

    // r[0x10] := 1024
    vm.instruction::<_, false>(op::addi(0x10, RegId::ZERO, alloc))
        .unwrap();
    vm.instruction::<_, false>(op::aloc(0x10)).unwrap();

    // r[0x20] := 128
    vm.instruction::<_, false>(op::addi(0x20, RegId::ZERO, 128))
        .unwrap();

    for i in 0..alloc {
        vm.instruction::<_, false>(op::addi(0x21, RegId::ZERO, i))
            .unwrap();
        vm.instruction::<_, false>(op::sb(RegId::HP, 0x21, i as Immediate12))
            .unwrap();
    }

    // r[0x23] := m[$hp, 0x20] == m[$zero, 0x20]
    vm.instruction::<_, false>(op::meq(0x23, RegId::HP, RegId::ZERO, 0x20))
        .unwrap();

    assert_eq!(0, vm.registers()[0x23]);

    // r[0x12] := $hp + r[0x20]
    vm.instruction::<_, false>(op::add(0x12, RegId::HP, 0x20))
        .unwrap();

    // Test ownership
    vm.instruction::<_, false>(op::mcp(RegId::HP, 0x12, 0x20))
        .unwrap();

    // r[0x23] := m[0x30, 0x20] == m[0x12, 0x20]
    vm.instruction::<_, false>(op::meq(0x23, RegId::HP, 0x12, 0x20))
        .unwrap();

    assert_eq!(1, vm.registers()[0x23]);

    // Assert ownership
    vm.instruction::<_, false>(op::subi(0x24, RegId::HP, 1))
        .unwrap(); // TODO: look into this
    let ownership_violated = vm.instruction::<_, false>(op::mcp(0x24, 0x12, 0x20));

    assert!(ownership_violated.is_err());

    // Assert no panic on overlapping
    vm.instruction::<_, false>(op::subi(0x25, 0x12, 1)).unwrap();
    let overlapping = vm.instruction::<_, false>(op::mcp(RegId::HP, 0x25, 0x20));

    assert!(overlapping.is_err());
}

#[test]
fn stack_alloc_ownership() {
    let mut vm = Interpreter::<_, _, _>::with_memory_storage();
    let gas_price = 0;
    let consensus_params = ConsensusParameters::standard();

    let tx = TransactionBuilder::script(vec![], vec![])
        .script_gas_limit(1000000)
        .add_fee_input()
        .finalize()
        .into_checked(Default::default(), &ConsensusParameters::standard())
        .expect("Empty script should be valid")
        .into_ready(
            gas_price,
            consensus_params.gas_costs(),
            consensus_params.fee_params(),
            None,
        )
        .unwrap();
    vm.init_script(tx).expect("Failed to init VM");

    vm.instruction::<_, false>(op::move_(0x10, RegId::SP))
        .unwrap();
    vm.instruction::<_, false>(op::cfei(2)).unwrap();

    // Assert allocated stack is writable
    vm.instruction::<_, false>(op::mcli(0x10, 2)).unwrap();
}

#[test_case(
    OwnershipRegisters::test(0..0, 0..0), 0..0
    => true; "empty mem range"
)]
#[test_case(
    OwnershipRegisters::test(0..0, 0..0), 0..0
    => true; "empty mem range (external)"
)]
#[test_case(
    OwnershipRegisters::test(0..0, 0..0), 0..1
    => false; "empty stack and heap"
)]
#[test_case(
    OwnershipRegisters::test(0..1, 0..0), 0..1
    => true; "in range for stack"
)]
#[test_case(
    OwnershipRegisters::test(0..1, 0..0), 0..2
    => false; "above stack range"
)]
#[test_case(
    OwnershipRegisters::test(0..0, 0..2), 1..2
    => true; "in range for heap"
)]
#[test_case(
    OwnershipRegisters::test(0..2, 1..2), 0..2
    => true; "crosses stack and heap"
)]
#[test_case(
    OwnershipRegisters::test(0..0, 9..10), 1..9
    => false; "not owned in Script context"
)]
#[test_case(
    OwnershipRegisters::test(0..0, 9..10), 1..9
    => false; "not owned in Call context"
)]
#[test_case(
    OwnershipRegisters::test(1_000_000..1_100_000, 5_900_000..6_300_000),
    999_000..7_100_200 => false; "crosses heap and stack range"
)]
#[test_case(
    OwnershipRegisters::test(0..20, 40..50),
    0..20 => true; "start inclusive and end exclusive"
)]
#[test_case(
    OwnershipRegisters::test(0..20, 40..50),
    20..41 => false; "start exclusive and end inclusive"
)]
#[test_case(
    OwnershipRegisters::test(0..0, VM_MAX_RAM..VM_MAX_RAM),
    MEM_SIZE..MEM_SIZE => true; "empty range at $hp (VM_MAX_RAM) should be allowed"
)]
#[test_case(
    OwnershipRegisters::test(0..0, 100..VM_MAX_RAM),
    100..100 => true; "empty range at $hp (not VM_MAX_RAM) should be allowed"
)]
fn test_ownership(reg: OwnershipRegisters, range: Range<usize>) -> bool {
    reg.verify_ownership(&MemoryRange::new(range.start, range.len()))
        .is_ok()
}

fn set_index(index: usize, val: u8, mut array: [u8; 100]) -> [u8; 100] {
    array[index] = val;
    array
}

#[test_case(
    1, & [],
    OwnershipRegisters::test(0..1, 100..100)
    => (false, [0u8; 100]); "External errors when write is empty"
)]
#[test_case(
    1, & [],
    OwnershipRegisters::test(0..1, 100..100)
    => (false, [0u8; 100]); "Internal errors when write is empty"
)]
#[test_case(
    1, & [2],
    OwnershipRegisters::test(0..2, 100..100)
    => (true, set_index(1, 2, [0u8; 100])); "External writes to stack"
)]
#[test_case(
    98, & [2],
    OwnershipRegisters::test(0..2, 97..100)
    => (true, set_index(98, 2, [0u8; 100])); "External writes to heap"
)]
#[test_case(
    1, & [2],
    OwnershipRegisters::test(0..2, 100..100)
    => (true, set_index(1, 2, [0u8; 100])); "Internal writes to stack"
)]
#[test_case(
    98, & [2],
    OwnershipRegisters::test(0..2, 97..100)
    => (true, set_index(98, 2, [0u8; 100])); "Internal writes to heap"
)]
#[test_case(
    1, & [2; 50],
    OwnershipRegisters::test(0..40, 100..100)
    => (false, [0u8; 100]); "External too large for stack"
)]
#[test_case(
    1, & [2; 50],
    OwnershipRegisters::test(0..40, 100..100)
    => (false, [0u8; 100]); "Internal too large for stack"
)]
#[test_case(
    61, & [2; 50],
    OwnershipRegisters::test(0..0, 60..100)
    => (false, [0u8; 100]); "Internal too large for heap"
)]
fn test_mem_write(
    addr: usize,
    data: &[u8],
    owner: OwnershipRegisters,
) -> (bool, [u8; 100]) {
    let mut memory: MemoryInstance = vec![0u8; MEM_SIZE].try_into().unwrap();
    let r = match memory.write(owner, addr, data.len()) {
        Ok(target) => {
            target.copy_from_slice(data);
            true
        }
        Err(_) => false,
    };
    (r, memory.read_bytes(0).unwrap())
}

// Zero-sized write
#[test_case(0, 0, 0, &[1, 2, 3, 4] => (true, [0xff, 0xff, 0xff, 0xff, 0xff]))]
#[test_case(1, 0, 0, &[1, 2, 3, 4] => (true, [0xff, 0xff, 0xff, 0xff, 0xff]))]
#[test_case(0, 0, 1, &[1, 2, 3, 4] => (true, [0xff, 0xff, 0xff, 0xff, 0xff]))]
// Dst address checks
#[test_case(0, 4, 0, &[1, 2, 3, 4] => (true, [1, 2, 3, 4, 0xff]))]
#[test_case(1, 4, 0, &[1, 2, 3, 4] => (true, [0xff, 1, 2, 3, 4]))]
#[test_case(2, 4, 0, &[1, 2, 3, 4] => (true, [0xff, 0xff, 1, 2, 3]))]
#[test_case(2, 2, 0, &[1, 2, 3, 4] => (true, [0xff, 0xff, 1, 2, 0xff]))]
// Zero padding when exceeding slice size
#[test_case(0, 2, 2, &[1, 2, 3, 4] => (true, [3, 4, 0xff, 0xff, 0xff]))]
#[test_case(0, 2, 3, &[1, 2, 3, 4] => (true, [4, 0, 0xff, 0xff, 0xff]))]
#[test_case(0, 2, 4, &[1, 2, 3, 4] => (true, [0, 0, 0xff, 0xff, 0xff]))]
#[test_case(0, 2, 5, &[1, 2, 3, 4] => (true, [0, 0, 0xff, 0xff, 0xff]))]
// Zero padding when exceeding slice size, but with nonzero dst address
#[test_case(1, 2, 2, &[1, 2, 3, 4] => (true, [0xff, 3, 4, 0xff, 0xff]))]
#[test_case(1, 2, 3, &[1, 2, 3, 4] => (true, [0xff, 4, 0, 0xff, 0xff]))]
#[test_case(1, 2, 4, &[1, 2, 3, 4] => (true, [0xff, 0, 0, 0xff, 0xff]))]
#[test_case(1, 2, 5, &[1, 2, 3, 4] => (true, [0xff, 0, 0, 0xff, 0xff]))]
// Zero-sized src slice
#[test_case(1, 0, 0, &[] => (true, [0xff, 0xff, 0xff, 0xff, 0xff]))]
#[test_case(1, 2, 0, &[] => (true, [0xff, 0, 0, 0xff, 0xff]))]
#[test_case(1, 2, 1, &[] => (true, [0xff, 0, 0, 0xff, 0xff]))]
#[test_case(1, 2, 2, &[] => (true, [0xff, 0, 0, 0xff, 0xff]))]
#[test_case(1, 2, 3, &[] => (true, [0xff, 0, 0, 0xff, 0xff]))]
fn test_copy_from_storage_zero_fill(
    addr: usize,
    len: usize,
    src_offset: Word,
    src_data: &[u8],
) -> (bool, [u8; 5]) {
    let contract_id = ContractId::zeroed();
    let contract_size = src_data.len();
    let mut storage = MemoryStorage::default();
    storage
        .storage_contract_insert(&contract_id, src_data)
        .unwrap();

    let mut memory: MemoryInstance = vec![0xffu8; MEM_SIZE].try_into().unwrap();
    let r = copy_from_storage_zero_fill::<ContractsRawCode, _>(
        &mut memory,
        OwnershipRegisters::test_full_stack(),
        &storage,
        addr as Word,
        len as Word,
        &contract_id,
        src_offset,
        contract_size,
        PanicReason::ContractNotFound,
    )
    .is_ok();
    let memory: [u8; 5] = memory[..5].try_into().unwrap();
    (r, memory)
}

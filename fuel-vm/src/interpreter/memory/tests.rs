use super::*;
use crate::prelude::*;
use fuel_asm::{op, RegId};

#[test]
fn alloc_pages() {
    const PAGE_SIZE: Word = VM_PAGE_SIZE as Word;
    let mut mem = VmMemory::new();
    assert_eq!(mem.update_allocations(0, VM_MAX_RAM).unwrap(), AllocatedPages(0));
    assert_eq!(mem.update_allocations(1, VM_MAX_RAM).unwrap(), AllocatedPages(1));
    assert_eq!(mem.update_allocations(1, VM_MAX_RAM).unwrap(), AllocatedPages(0));
    assert_eq!(mem.update_allocations(1, VM_MAX_RAM - 1).unwrap(), AllocatedPages(1));
    assert_eq!(mem.update_allocations(1, VM_MAX_RAM - 1).unwrap(), AllocatedPages(0));
    assert_eq!(mem.update_allocations(0, VM_MAX_RAM - 1).unwrap(), AllocatedPages(0));
    assert_eq!(mem.update_allocations(1, VM_MAX_RAM - 1).unwrap(), AllocatedPages(0));
    assert_eq!(mem.update_allocations(1, VM_MAX_RAM).unwrap(), AllocatedPages(0));
    assert_eq!(mem.update_allocations(1, VM_MAX_RAM - 1).unwrap(), AllocatedPages(0));
    assert_eq!(
        mem.update_allocations(PAGE_SIZE, VM_MAX_RAM - 1).unwrap(),
        AllocatedPages(0)
    );
    assert_eq!(
        mem.update_allocations(PAGE_SIZE + 1, VM_MAX_RAM - 1).unwrap(),
        AllocatedPages(1)
    );
    assert_eq!(
        mem.update_allocations(PAGE_SIZE * 2, VM_MAX_RAM - 1).unwrap(),
        AllocatedPages(0)
    );
    assert_eq!(
        mem.update_allocations(PAGE_SIZE * 2 + 1, VM_MAX_RAM - 1).unwrap(),
        AllocatedPages(1)
    );
}

#[test]
fn memcopy() {
    let mut vm = Interpreter::with_memory_storage();
    let params = ConsensusParameters::default().with_max_gas_per_tx(Word::MAX / 2);
    let tx = TransactionBuilder::script(op::ret(0x10).to_bytes().to_vec(), vec![])
        .gas_limit(params.max_gas_per_tx)
        .add_random_fee_input()
        .finalize();

    let tx = tx
        .into_checked(Default::default(), &params, vm.gas_costs())
        .expect("default tx should produce a valid checked transaction");

    vm.init_script(tx).expect("Failed to init VM");

    let alloc = 1024;

    // r[0x10] := 1024
    vm.instruction(op::addi(0x10, RegId::ZERO, alloc)).unwrap();
    vm.instruction(op::aloc(0x10)).unwrap();

    // r[0x20] := 128
    vm.instruction(op::addi(0x20, RegId::ZERO, 128)).unwrap();

    for i in 0..alloc {
        vm.instruction(op::addi(0x21, RegId::ZERO, i)).unwrap();
        vm.instruction(op::sb(RegId::HP, 0x21, i as Immediate12)).unwrap();
    }

    // r[0x23] := m[$hp, 0x20] == m[$zero, 0x20]
    vm.instruction(op::meq(0x23, RegId::HP, RegId::ZERO, 0x20)).unwrap();

    assert_eq!(0, vm.registers()[0x23]);

    // r[0x12] := $hp + r[0x20]
    vm.instruction(op::add(0x12, RegId::HP, 0x20)).unwrap();

    // Test ownership
    vm.instruction(op::mcp(RegId::HP, 0x12, 0x20)).unwrap();

    // r[0x23] := m[0x30, 0x20] == m[0x12, 0x20]
    vm.instruction(op::meq(0x23, RegId::HP, 0x12, 0x20)).unwrap();

    assert_eq!(1, vm.registers()[0x23]);

    // Assert ownership
    vm.instruction(op::subi(0x24, RegId::HP, 1)).unwrap(); // TODO: look into this
    let ownership_violated = vm.instruction(op::mcp(0x24, 0x12, 0x20));

    assert!(ownership_violated.is_err());

    // Assert no panic on overlapping
    vm.instruction(op::subi(0x25, 0x12, 1)).unwrap();
    let overlapping = vm.instruction(op::mcp(RegId::HP, 0x25, 0x20));

    assert!(overlapping.is_err());
}

#[test]
fn stack_alloc_ownership() {
    let mut vm = Interpreter::with_memory_storage();

    let tx = TransactionBuilder::script(vec![], vec![])
        .gas_limit(1000000)
        .add_random_fee_input()
        .finalize()
        .into_checked(Default::default(), &Default::default(), &Default::default())
        .expect("Empty script should be valid");
    vm.init_script(tx).expect("Failed to init VM");

    vm.instruction(op::move_(0x10, RegId::SP)).unwrap();
    vm.instruction(op::cfei(2)).unwrap();

    // Assert allocated stack is writable
    vm.instruction(op::mcli(0x10, 2)).unwrap();
}

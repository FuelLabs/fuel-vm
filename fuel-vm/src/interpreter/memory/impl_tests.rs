#![allow(clippy::cast_possible_truncation)]

use crate::constraints::reg_key::*;

use super::{
    Memory,
    Reg,
    VM_MAX_RAM,
};

use alloc::vec;

#[test]
fn grow_heap_over_old_stack() {
    let mut memory = Memory::new();

    memory.grow_stack(VM_MAX_RAM - 100).expect("Can grow stack");

    memory
        .write_noownerchecks(0, VM_MAX_RAM - 100)
        .unwrap()
        .fill(1u8);

    // Now extend heap to the same location, so that whole memory is allocated
    memory
        .grow_heap(Reg::<SP>::new(&(VM_MAX_RAM - 100)), VM_MAX_RAM - 100)
        .expect("Can grow heap");

    memory
        .write_noownerchecks(VM_MAX_RAM - 100, 100)
        .unwrap()
        .fill(2u8);

    // Now try to extend heap over old stack. This should fail
    memory
        .grow_heap(Reg::<SP>::new(&(VM_MAX_RAM - 100)), VM_MAX_RAM - 101)
        .expect_err("Cannot grow heap over stack");

    // Check that memory is still intact
    assert_eq!(
        memory.read(0, VM_MAX_RAM - 100).unwrap().to_vec(),
        vec![1u8; (VM_MAX_RAM - 100) as usize]
    );
    assert_eq!(
        memory.read(VM_MAX_RAM - 100, 100).unwrap().to_vec(),
        vec![2u8; 100]
    );

    // Now extend heap over stack after (conceptually) shrinking it
    memory
        .grow_heap(Reg::<SP>::new(&(VM_MAX_RAM - 200)), VM_MAX_RAM - 200)
        .expect("Should be able to grow heap");

    // Check that memory is still intact for the old stack
    assert_eq!(
        memory.read(0, VM_MAX_RAM - 200).unwrap().to_vec(),
        vec![1u8; (VM_MAX_RAM - 200) as usize]
    );

    // New memory should be zeroed
    assert_eq!(
        memory.read(VM_MAX_RAM - 200, 100).unwrap().to_vec(),
        vec![0u8; 100]
    );

    // And the rest should be intact
    assert_eq!(memory.read(VM_MAX_RAM - 100, 100).unwrap(), vec![2u8; 100]);
}

#[test]
fn reads_cannot_cross_from_stack_to_heap() {
    let mut memory = Memory::new();

    // Allocate the whole stack
    let partition = VM_MAX_RAM - 100;
    memory.grow_stack(partition).expect("Can grow stack");
    memory
        .grow_heap(Reg::<SP>::new(&partition), partition)
        .expect("Can grow heap");

    // Attempt cross-partition operations
    memory
        .read(partition - 2, 4)
        .expect_err("Cannot read across stack/heap boundary");
    memory
        .write_noownerchecks(partition - 2, 4)
        .expect_err("Cannot read across stack/heap boundary");
}

#[test]
fn reading_from_internally_allocated_heap_below_hp_fails() {
    let mut memory = Memory::new();

    // Allocate small heap
    memory
        .grow_heap(Reg::<SP>::new(&0), VM_MAX_RAM - 10)
        .expect("Can grow heap");

    // Attempt to read the heap that's now allocated internally but is not accessible
    memory
        .write_noownerchecks(VM_MAX_RAM - 16, 16)
        .expect_err("Cannot read across stack/heap boundary");
}

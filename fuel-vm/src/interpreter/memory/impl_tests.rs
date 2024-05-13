#![allow(clippy::cast_possible_truncation)]

use crate::{
    constraints::reg_key::*,
    consts::MEM_SIZE,
};

use super::{
    Memory,
    Reg,
    VM_MAX_RAM,
};

use alloc::vec;

#[test]
fn grow_heap_over_old_stack() {
    let mut memory = Memory::new();

    let sp = VM_MAX_RAM - 100;
    let mut hp = VM_MAX_RAM;

    memory.grow_stack(sp).expect("Can grow stack");

    memory.write_noownerchecks(0, sp).unwrap().fill(1u8);

    // Now extend heap to the same location, so that whole memory is allocated
    memory
        .grow_heap_by(Reg::<SP>::new(&sp), RegMut::<HP>::new(&mut hp), 100)
        .expect("Can grow heap");

    memory.write_noownerchecks(hp, 100).unwrap().fill(2u8);

    // Now try to extend heap over old stack. This should fail
    memory
        .grow_heap_by(Reg::<SP>::new(&sp), RegMut::<HP>::new(&mut hp), 1)
        .expect_err("Cannot grow heap over stack");

    // Check that memory is still intact
    assert_eq!(memory.read(0, hp).unwrap().to_vec(), vec![1u8; hp as usize]);
    assert_eq!(memory.read(hp, 100).unwrap().to_vec(), vec![2u8; 100]);

    // Now extend heap over stack after (conceptually) shrinking it
    let sp = VM_MAX_RAM - 200;
    memory
        .grow_heap_by(Reg::<SP>::new(&sp), RegMut::<HP>::new(&mut hp), 100)
        .expect("Should be able to grow heap");

    // Check that memory is still intact for the old stack
    assert_eq!(memory.read(0, hp).unwrap().to_vec(), vec![1u8; hp as usize]);

    // New memory should be zeroed
    assert_eq!(memory.read(hp, 100).unwrap().to_vec(), vec![0u8; 100]);

    // And the rest should be intact
    assert_eq!(memory.read(VM_MAX_RAM - 100, 100).unwrap(), vec![2u8; 100]);
}

#[test]
fn reads_cannot_cross_from_stack_to_heap() {
    let mut memory = Memory::new();

    // Allocate the whole stack
    let sp = VM_MAX_RAM - 100;
    let mut hp = VM_MAX_RAM;
    memory.grow_stack(sp).expect("Can grow stack");
    memory
        .grow_heap_by(Reg::<SP>::new(&sp), RegMut::<HP>::new(&mut hp), 100)
        .expect("Can grow heap");
    assert_eq!(hp, VM_MAX_RAM - 100);

    // Attempt cross-partition operations
    memory
        .read(sp - 2, 4)
        .expect_err("Cannot read across stack/heap boundary");
    memory
        .write_noownerchecks(sp - 2, 4)
        .expect_err("Cannot write across stack/heap boundary");
}

#[test]
fn reading_from_internally_allocated_heap_below_hp_fails() {
    let mut memory = Memory::new();

    // Allocate small heap
    let mut hp = VM_MAX_RAM;
    memory
        .grow_heap_by(Reg::<SP>::new(&0), RegMut::<HP>::new(&mut hp), 10)
        .expect("Can grow heap");

    // Attempt to read the heap that's now allocated internally but is not accessible
    memory
        .write_noownerchecks(VM_MAX_RAM - 16, 16)
        .expect_err("Cannot read across stack/heap boundary");
}

#[test]
fn memory_reset() {
    let mut memory = Memory::new();

    memory.grow_stack(10).unwrap();
    memory.read(0, 1).expect("Stack should be nonempty");
    memory.reset();
    memory.read(0, 1).expect_err("Stack should be empty");

    let mut hp = VM_MAX_RAM;
    memory
        .grow_heap_by(Reg::<SP>::new(&0), RegMut::<HP>::new(&mut hp), 10)
        .unwrap();
    assert_eq!(hp, VM_MAX_RAM - 10);
    memory
        .read(VM_MAX_RAM - 1, 1)
        .expect("Heap should be nonempty");
    memory.reset();
    memory
        .read(VM_MAX_RAM - 1, 1)
        .expect_err("Heap should be empty");
    assert_eq!(memory.hp, MEM_SIZE);
}

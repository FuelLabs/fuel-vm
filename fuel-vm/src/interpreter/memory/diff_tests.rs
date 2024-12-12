#![allow(clippy::cast_possible_truncation)] // test code

use proptest::prelude::*;

use crate::{
    consts::{
        MEM_SIZE,
        VM_MAX_RAM,
    },
    interpreter::{
        memory::OwnershipRegisters,
        MemorySliceChange,
        Reg,
        RegMut,
    },
};

use super::MemoryInstance;

#[test]
fn empty_memories_produces_empty_diff() {
    let memory1 = MemoryInstance::new();
    let memory2 = MemoryInstance::new();
    let diff = memory1.diff_patches(&memory2);
    assert!(diff.is_empty());
}

#[test]
fn stack_and_heap_produce_patches() {
    let all_memory_owned =
        OwnershipRegisters::test(0..VM_MAX_RAM - 4, VM_MAX_RAM - 4..VM_MAX_RAM);

    let mut memory1 = MemoryInstance::new();
    let mut hp = VM_MAX_RAM;
    memory1.grow_stack(4).unwrap();
    memory1
        .grow_heap_by(Reg::new(&4), RegMut::new(&mut hp), 4)
        .unwrap();

    let mut memory2 = memory1.clone();
    memory2
        .write_bytes(all_memory_owned, 0, [1, 2, 3, 4])
        .unwrap();
    memory2
        .write_bytes(all_memory_owned, VM_MAX_RAM - 4, [5, 6, 7, 8])
        .unwrap();

    let diff = memory1.diff_patches(&memory2);
    assert_eq!(
        diff,
        vec![
            MemorySliceChange {
                global_start: 0,
                data: vec![1, 2, 3, 4],
            },
            MemorySliceChange {
                global_start: MEM_SIZE - 4,
                data: vec![5, 6, 7, 8],
            },
        ]
    );
}

#[test]
fn overwriting_produces_patch() {
    let all_memory_owned =
        OwnershipRegisters::test(0..VM_MAX_RAM, VM_MAX_RAM..VM_MAX_RAM);

    let mut memory1 = MemoryInstance::new();
    let mut hp = VM_MAX_RAM;
    memory1.grow_stack(4).unwrap();
    memory1
        .grow_heap_by(Reg::new(&4), RegMut::new(&mut hp), 4)
        .unwrap();

    let mut memory2 = memory1.clone();
    memory2
        .write_bytes(all_memory_owned, 0, [1, 2, 3, 4])
        .unwrap();
    let diff1 = memory1.diff_patches(&memory2);

    let mut memory3 = memory2.clone();
    memory3
        .write_bytes(all_memory_owned, 0, [5, 6, 7, 8])
        .unwrap();
    let diff2 = memory2.diff_patches(&memory3);

    assert_eq!(
        diff1,
        vec![MemorySliceChange {
            global_start: 0,
            data: vec![1, 2, 3, 4],
        }]
    );
    assert_eq!(
        diff2,
        vec![MemorySliceChange {
            global_start: 0,
            data: vec![5, 6, 7, 8],
        }]
    );
}

#[test]
fn unchanged_write_doesnt_produce_patch() {
    let all_memory_owned =
        OwnershipRegisters::test(0..VM_MAX_RAM, VM_MAX_RAM..VM_MAX_RAM);

    let mut memory1 = MemoryInstance::new();
    memory1.grow_stack(8).unwrap();

    let mut memory2 = memory1.clone();
    memory2
        .write_bytes(all_memory_owned, 0, [1, 2, 3, 4])
        .unwrap();
    let diff1 = memory1.diff_patches(&memory2);

    let mut memory3 = memory2.clone();
    memory3
        .write_bytes(all_memory_owned, 0, [1, 2, 3, 4])
        .unwrap();
    let diff2 = memory2.diff_patches(&memory3);

    assert_eq!(
        diff1,
        vec![MemorySliceChange {
            global_start: 0,
            data: vec![1, 2, 3, 4],
        }]
    );
    assert!(diff2.is_empty());
}

#[test]
fn overlapping_overwrite_produces_patch() {
    let all_memory_owned =
        OwnershipRegisters::test(0..VM_MAX_RAM, VM_MAX_RAM..VM_MAX_RAM);

    let mut memory1 = MemoryInstance::new();
    memory1.grow_stack(8).unwrap();

    memory1
        .write_bytes(all_memory_owned, 0, [1, 2, 3, 4])
        .unwrap();

    let mut memory2 = memory1.clone();
    memory2
        .write_bytes(all_memory_owned, 2, [5, 6, 7, 8])
        .unwrap();

    let diff = memory1.diff_patches(&memory2);

    assert_eq!(
        diff,
        vec![MemorySliceChange {
            global_start: 2,
            data: vec![5, 6, 7, 8],
        }]
    );
}

#[test]
fn stack_retraction_overwrite_produces_correct_patch() {
    let split_change = 10;
    let split_a = VM_MAX_RAM / 2 + split_change;
    let split_b = VM_MAX_RAM / 2;

    let split_a_owner = OwnershipRegisters::test(0..split_a, split_a..VM_MAX_RAM);
    let split_b_owner = OwnershipRegisters::test(0..split_b, split_b..VM_MAX_RAM);

    // Allocate regions
    let mut memory1 = MemoryInstance::new();
    let mut hp = VM_MAX_RAM;
    memory1.grow_stack(split_a).unwrap();
    memory1
        .grow_heap_by(
            Reg::new(&split_a),
            RegMut::new(&mut hp),
            VM_MAX_RAM - split_a,
        )
        .unwrap();

    // Fill stack with 0x01 and heap with 0x02
    memory1.write(split_a_owner, 0, split_a).unwrap().fill(0x01);
    memory1
        .write(split_a_owner, split_a, VM_MAX_RAM - split_a)
        .unwrap()
        .fill(0x02);

    // Check that we generate the correct patch from empty state to this
    let diff = MemoryInstance::new().diff_patches(&memory1);
    assert_eq!(diff.len(), 1);
    assert_eq!(diff[0].global_start, 0);
    assert_eq!(diff[0].data.len(), MEM_SIZE);
    assert!(diff[0].data[..split_a as usize].iter().all(|&x| x == 0x01));
    assert!(diff[0].data[split_a as usize..].iter().all(|&x| x == 0x02));

    // Now we will implicity shrink the stack by explictly expanding the heap over it.
    // This keeps the underlying stack allocation, which is subtle and could cause bugs.
    let mut memory2 = memory1.clone();
    memory2
        .grow_heap_by(Reg::new(&split_b), RegMut::new(&mut hp), split_change)
        .unwrap();

    // Check that we generate the correct patch for this change,
    // as the newly allocated heap memory should be is zeroed.
    let diff = memory1.diff_patches(&memory2);
    assert_eq!(
        diff,
        vec![MemorySliceChange {
            global_start: split_b as usize,
            data: vec![0; split_change as usize],
        }]
    );

    // And now we overwrite the whole heap with 0x03 to check that we generate the correct
    // patch.
    memory2
        .write(split_b_owner, split_b, VM_MAX_RAM - split_b)
        .unwrap()
        .fill(0x03);
    let diff = memory1.diff_patches(&memory2);
    assert_eq!(diff.len(), 1);
    assert_eq!(diff[0].global_start, split_b as usize);
    assert_eq!(diff[0].data.len(), MEM_SIZE - split_b as usize);
    assert!(diff[0].data.iter().all(|&x| x == 0x03));
}

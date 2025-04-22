#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

use alloc::vec;

use fuel_asm::{
    Imm24,
    PanicReason,
};

use crate::{
    constraints::reg_key::*,
    consts::*,
    error::PanicOrBug,
    interpreter::memory::{
        MemoryInstance,
        pop_selected_registers,
        push_selected_registers,
    },
};

#[rstest::rstest]
fn test_push_pop(
    #[values(ProgramRegistersSegment::Low, ProgramRegistersSegment::High)]
    segment: ProgramRegistersSegment,
    #[values(
        0,
        0b01,
        0b10,
        0b11,
        0b_100000_000000_000000_000000,
        0b_111100_011101_101011_001100,
        0b_111100_011101_101011_001101,
        Imm24::MAX.into(),
    )]
    bitmask: u32,
) {
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut pc = 0;
    let mut sp = 0;

    let orig_regs = [2; VM_REGISTER_PROGRAM_COUNT];
    let mut reg_values = orig_regs;
    let mut regs = ProgramRegisters(&mut reg_values);

    let popcnt = bitmask.count_ones() as u64;
    let bitmask = Imm24::new_checked(bitmask).unwrap();

    // Save registers to stack
    push_selected_registers(
        &mut memory,
        RegMut::new(&mut sp),
        Reg::new(&0),
        Reg::new(&VM_MAX_RAM),
        RegMut::new(&mut pc),
        &regs,
        segment,
        bitmask,
    )
    .expect("Push failed unexpectedly");

    assert_eq!(sp, popcnt * 8);
    assert_eq!(pc, 4);

    // Clear registers
    regs.0.fill(0);

    // Restore registers
    pop_selected_registers(
        &mut memory,
        RegMut::new(&mut sp),
        Reg::new(&0),
        Reg::new(&VM_MAX_RAM),
        RegMut::new(&mut pc),
        &mut regs,
        segment,
        bitmask,
    )
    .expect("Pop failed unexpectedly");

    assert_eq!(sp, 0);
    assert_eq!(pc, 8);

    // Make sure that the correct number of registers were restored
    assert_eq!(
        orig_regs
            .iter()
            .zip(reg_values.iter())
            .filter(|(a, b)| a == b)
            .count() as u64,
        popcnt
    );
}

#[test]
fn test_push_stack_overflow() {
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut pc = 0;
    let mut sp = 10;
    let hp = 14;

    let mut reg_values = [0; VM_REGISTER_PROGRAM_COUNT];
    let regs = ProgramRegisters(&mut reg_values);

    let result = push_selected_registers(
        &mut memory,
        RegMut::new(&mut sp),
        Reg::new(&10),
        Reg::new(&hp),
        RegMut::new(&mut pc),
        &regs,
        ProgramRegistersSegment::Low,
        Imm24::new(1),
    );

    assert_eq!(
        result,
        Err(PanicOrBug::Panic(PanicReason::MemoryGrowthOverlap))
    );
}

#[test]
fn test_pop_from_empty_stack() {
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut pc = 0;
    let mut sp = 32;
    let ssp = 16;

    let mut reg_values = [0; VM_REGISTER_PROGRAM_COUNT];
    let mut regs = ProgramRegisters(&mut reg_values);

    let result = pop_selected_registers(
        &mut memory,
        RegMut::new(&mut sp),
        Reg::new(&ssp),
        Reg::new(&VM_MAX_RAM),
        RegMut::new(&mut pc),
        &mut regs,
        ProgramRegistersSegment::Low,
        Imm24::new(0b111),
    );

    assert_eq!(result, Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)));
}

#[test]
fn test_pop_sp_overflow() {
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut pc = 0;
    let mut sp = 16;
    let ssp = 0;

    let mut reg_values = [0; VM_REGISTER_PROGRAM_COUNT];
    let mut regs = ProgramRegisters(&mut reg_values);

    let result = pop_selected_registers(
        &mut memory,
        RegMut::new(&mut sp),
        Reg::new(&ssp),
        Reg::new(&VM_MAX_RAM),
        RegMut::new(&mut pc),
        &mut regs,
        ProgramRegistersSegment::Low,
        Imm24::new(0b111),
    );

    assert_eq!(result, Err(PanicOrBug::Panic(PanicReason::MemoryOverflow)));
}

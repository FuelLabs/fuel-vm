#![cfg(feature = "std")]

use fuel_asm::{
    Imm18,
    Instruction,
    PanicReason,
    RegId,
    op,
};
use fuel_tx::Receipt;
use rstest::rstest;

use super::test_helpers::{
    assert_panics,
    assert_success,
    run_script,
};

#[rstest]
fn relative_jump_forwards(
    #[values(0, 1, 2)] offset: u32,
    #[values("jmp", "jnz", "jne")] condition: &'static str,
) {
    #[rustfmt::skip]
    let script = vec![
        op::movi(0x20, 2 - offset),
        match condition {
            "jmp" => op::jmpf(0x20, offset),
            "jnz" => op::jnzf(RegId::ONE, 0x20, offset as _),
            "jne" => op::jnef(RegId::ZERO, RegId::ONE, 0x20, offset as _),
            _ => unreachable!(),
        },
        op::rvrt(RegId::ONE),
        op::rvrt(RegId::ONE),
        op::ret(RegId::ONE),
    ];

    let receipts = run_script(script);
    assert_success(&receipts);
}

#[rstest]
fn relative_jump_backwards(
    #[values(0, 1, 2)] offset: u32,
    #[values("jmp", "jnz", "jne")] condition: &'static str,
) {
    #[rustfmt::skip]
    let script = vec![
        op::movi(0x20, 2 - offset),
        op::jmpf(RegId::ZERO, 5),
        op::rvrt(RegId::ONE),
        op::rvrt(RegId::ONE),
        op::ret(RegId::ONE),
        op::rvrt(RegId::ONE),
        op::rvrt(RegId::ONE),
        match condition {
            "jmp" => op::jmpb(0x20, offset),
            "jnz" => op::jnzb(RegId::ONE, 0x20, offset as _),
            "jne" => op::jneb(RegId::ZERO, RegId::ONE, 0x20, offset as _),
            _ => unreachable!(),
        },
        op::rvrt(RegId::ONE),
    ];

    let receipts = run_script(script);
    assert_success(&receipts);
}

#[rstest]
fn relative_jump_condition_false(
    #[values(
        op::jnzb(RegId::ZERO, RegId::ZERO, 2),
        op::jnzf(RegId::ZERO, RegId::ZERO, 2),
        op::jneb(RegId::ZERO, RegId::ZERO, RegId::ZERO, 2),
        op::jnef(RegId::ZERO, RegId::ZERO, RegId::ZERO, 2)
    )]
    instr: Instruction,
) {
    #[rustfmt::skip]
    let script = vec![
        instr,
        op::ret(RegId::ONE),
    ];

    let receipts = run_script(script);
    assert_success(&receipts);
}

#[test]
fn relative_jump_dynamic_below_zero() {
    #[rustfmt::skip]
    let script = vec![
        op::not(0x20, RegId::ZERO),
        op::jmpb(0x20, 0),
        op::ret(RegId::ONE),
    ];

    let receipts = run_script(script);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[test]
fn relative_jump_immediate_below_zero() {
    #[rustfmt::skip]
    let script = vec![
        op::jmpb(RegId::ZERO, Imm18::MAX.into()),
        op::ret(RegId::ONE),
    ];

    let receipts = run_script(script);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[test]
fn relative_jump_subtract_underflow() {
    #[rustfmt::skip]
    let script = vec![
        op::not(0x20, RegId::ZERO),
        op::jmpb(0x20, Imm18::MAX.into()),
        op::ret(RegId::ONE),
    ];

    let receipts = run_script(script);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[test]
fn relative_jump_dynamic_above_ram() {
    #[rustfmt::skip]
    let script = vec![
        op::not(0x20, RegId::ZERO),
        op::jmpf(0x20, 0),
        op::ret(RegId::ONE),
    ];

    let receipts = run_script(script);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[test]
fn relative_jump_dynamic_overflow() {
    #[rustfmt::skip]
    let script = vec![
        op::not(0x20, RegId::ZERO),
        op::jmpf(0x20, Imm18::MAX.into()),
        op::ret(RegId::ONE),
    ];

    let receipts = run_script(script);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[test]
fn relative_jump_repeat_loop() {
    let var_counter = RegId::new(0x20);

    #[rustfmt::skip]
    let script = vec![
        op::movi(var_counter, 42),
        // loop_start:
        op::log(var_counter, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::subi(var_counter, var_counter, 1),
        op::jnzb(var_counter, RegId::ZERO, 1), // if counter != 0 then goto loop_start
        op::ret(RegId::ONE),
    ];

    let receipts = run_script(script);
    assert_success(&receipts);

    for (i, receipt) in (0..=42).rev().zip(receipts.iter()) {
        if let Receipt::Log { ra, .. } = receipt {
            assert_eq!(*ra, i);
        }
    }
}

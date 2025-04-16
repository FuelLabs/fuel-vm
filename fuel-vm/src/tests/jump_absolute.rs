use alloc::vec;

use super::test_helpers::run_script;
use fuel_asm::{
    RegId,
    op,
};
use fuel_vm::prelude::*;

#[test]
fn jump_if_not_zero_immediate_jump() {
    #[rustfmt::skip]
    let script_jnzi_does_jump = vec![
        op::jnzi(RegId::ONE, 2), // Jump to last instr if reg one is zero
        op::rvrt(RegId::ONE),    // Revert
        op::ret(RegId::ONE),     // Return successfully
    ];

    let receipts = run_script(script_jnzi_does_jump);

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Return { .. }));
}

#[test]
fn jump_if_not_zero_immediate_no_jump() {
    #[rustfmt::skip]
    let script_jnzi_does_not_jump = vec![
        op::jnzi(RegId::ZERO, 2), // Jump to last instr if reg zero is zero
        op::rvrt(RegId::ONE),     // Revert
        op::ret(RegId::ONE),      // Return successfully
    ];

    let receipts = run_script(script_jnzi_does_not_jump);

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Revert { .. }));
}

#[test]
fn jump_dynamic() {
    #[rustfmt::skip]
    let script = vec![
        op::movi(RegId::WRITABLE, 3), // Jump target: last instr
        op::jmp(RegId::WRITABLE),     // Jump
        op::rvrt(RegId::ONE),         // Revert
        op::ret(RegId::ONE),          // Return successfully
    ];

    let receipts = run_script(script);

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Return { .. }));
}

#[test]
fn jump_dynamic_condition_true() {
    #[rustfmt::skip]
    let script = vec![
        op::movi(RegId::WRITABLE, 3),                              // Jump target: last instr
        op::jne(RegId::ZERO, RegId::ONE, RegId::WRITABLE), // Conditional jump (yes, because 0 != 1)
        op::rvrt(RegId::ONE),                                      // Revert
        op::ret(RegId::ONE),                                       // Return successfully
    ];

    let receipts = run_script(script);

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Return { .. }));
}

#[test]
fn jump_dynamic_condition_false() {
    #[rustfmt::skip]
    let script = vec![
        op::movi(RegId::WRITABLE, 3),                               // Jump target: last instr
        op::jne(RegId::ZERO, RegId::ZERO, RegId::WRITABLE), // Conditional jump (no, because 0 != 0)
        op::rvrt(RegId::ONE),                                       // Revert
        op::ret(RegId::ONE),                                        // Return successfully
    ];

    let receipts = run_script(script);

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Revert { .. }));
}

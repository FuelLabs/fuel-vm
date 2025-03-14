use crate::{
    prelude::*,
    tests::test_helpers::{
        assert_panics,
        assert_success,
        run_script,
    },
};
use fuel_asm::{
    op,
    RegId,
};

#[test]
fn jump_and_link__allows_discarding_return_address() {
    let script = vec![
        op::jal(RegId::ZERO, RegId::PC, 1), // Just jump to the next instruction
        op::ret(RegId::ONE),
    ];
    let receipts = run_script(script);
    assert_success(&receipts);
}

#[test]
fn jump_and_link__cannot_write_reserved_registers() {
    let script = vec![
        op::jal(RegId::ONE, RegId::PC, 1), // Just jump to the next instruction
        op::ret(RegId::ONE),
    ];
    let receipts = run_script(script);
    assert_panics(&receipts, PanicReason::ReservedRegisterNotWritable);
}

#[test]
fn jump_and_link__subroutine_call_works() {
    let reg_fn_addr = RegId::new(0x10);
    let reg_return_addr = RegId::new(0x11);
    let reg_tmp = RegId::new(0x12);

    let canary = 0x1337;

    let subroutine = vec![
        op::movi(reg_tmp, canary as _),
        op::log(reg_tmp, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::jal(RegId::ZERO, reg_return_addr, 0), // Return from the subroutine
    ];

    const MAIN_LEN: usize = 3; // Use a constant so we don't need to backpatch
    let mut script = vec![
        // Get current address so we know what location to call
        op::addi(reg_fn_addr, RegId::PC, (Instruction::SIZE * MAIN_LEN) as _),
        op::jal(reg_return_addr, reg_fn_addr, 0), // Call subroutine
        op::ret(RegId::ONE),                      // Return from the script
    ];
    assert_eq!(MAIN_LEN, script.len());
    script.extend(subroutine);

    let receipts = run_script(script);
    assert_success(&receipts);

    if let Some(Receipt::Log { ra, .. }) = receipts.get(0) {
        assert!(*ra == canary, "Expected canary value to be logged");
    } else {
        panic!("Expected a log receipt");
    };
}

#[test]
fn jump_and_link__immediate_count_is_instructions() {
    let reg_return_addr = RegId::new(0x11);
    let reg_tmp = RegId::new(0x12);

    let skip = 3; // Jump over the next 3 instructions

    let script = vec![
        op::movi(reg_tmp, 5),
        op::jal(reg_return_addr, RegId::PC, (skip + 1) as _), /* Zero would mean
                                                               * jumping to this
                                                               * instruction */
        op::subi(reg_tmp, reg_tmp, 1),
        op::subi(reg_tmp, reg_tmp, 1),
        op::subi(reg_tmp, reg_tmp, 1),
        op::subi(reg_tmp, reg_tmp, 1),
        op::subi(reg_tmp, reg_tmp, 1),
        op::log(reg_tmp, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ];

    let receipts = run_script(script);
    assert_success(&receipts);

    if let Some(Receipt::Log { ra, .. }) = receipts.get(0) {
        assert_eq!(*ra, skip, "Expected correct number of skipped instructions");
    } else {
        panic!("Expected a log receipt");
    };
}

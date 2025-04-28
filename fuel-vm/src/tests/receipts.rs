use fuel_asm::{
    PanicReason,
    RegId,
    op,
};
use fuel_tx::{
    Receipt,
    ScriptExecutionResult,
};

use alloc::vec;

use crate::interpreter::ReceiptsCtx;

use super::test_helpers::run_script;

#[test]
fn too_many_receipts_panics() {
    let receipts = run_script(vec![
        op::log(RegId::ZERO, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::jmpb(RegId::ZERO, 0),
    ]);

    assert_eq!(receipts.len(), ReceiptsCtx::MAX_RECEIPTS);

    // The panic receipt should have still been pushed correctly
    let Receipt::Panic { reason, .. } = receipts[ReceiptsCtx::MAX_RECEIPTS - 2] else {
        panic!("Expect panic receipt");
    };
    assert_eq!(*reason.reason(), PanicReason::TooManyReceipts);
}

#[test]
fn can_panic_just_before_max_receipts() {
    let receipts = run_script(vec![
        op::movi(0x10, ReceiptsCtx::MAX_RECEIPTS as u32 - 2),
        op::log(RegId::ZERO, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::subi(0x10, 0x10, 1),
        op::jnzb(0x10, RegId::ZERO, 1),
        op::div(0x10, RegId::ZERO, RegId::ZERO), // Divide by zero
    ]);

    assert_eq!(receipts.len(), ReceiptsCtx::MAX_RECEIPTS);

    // The panic receipt should have still been pushed correctly
    let Receipt::Panic { reason, .. } = receipts[ReceiptsCtx::MAX_RECEIPTS - 2] else {
        panic!("Expect panic receipt");
    };
    assert_eq!(*reason.reason(), PanicReason::ArithmeticError);
}

#[test]
fn can_return_successfully_just_below_max_receipts() {
    let receipts = run_script(vec![
        op::movi(0x10, ReceiptsCtx::MAX_RECEIPTS as u32 - 3),
        op::log(RegId::ZERO, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::subi(0x10, 0x10, 1),
        op::jnzb(0x10, RegId::ZERO, 1),
        op::ret(RegId::ONE),
    ]);

    assert_eq!(receipts.len(), ReceiptsCtx::MAX_RECEIPTS - 1);

    // The panic receipt should have still been pushed correctly
    let Receipt::ScriptResult { result, .. } = receipts[ReceiptsCtx::MAX_RECEIPTS - 2]
    else {
        panic!("Expect result receipt");
    };
    assert_eq!(result, ScriptExecutionResult::Success);
}

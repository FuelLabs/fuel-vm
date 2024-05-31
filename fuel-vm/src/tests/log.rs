use alloc::{
    vec,
    vec::Vec,
};

use super::test_helpers;
use fuel_asm::*;
use fuel_tx::Receipt;
use fuel_types::Immediate24;
use fuel_vm::consts::{
    VM_MAX_RAM,
    VM_REGISTER_COUNT,
};
use test_helpers::{
    assert_panics,
    assert_success,
    run_script,
    set_full_word,
};

#[test]
fn all_registers_can_be_logged() {
    let mut script = Vec::new();
    for reg in 0..(VM_REGISTER_COUNT as u8) {
        script.push(op::movi(0x30, reg as Immediate24));
        script.push(op::log(0x30, reg, RegId::ZERO, RegId::ZERO));
    }
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());
    assert_success(&receipts);

    let mut receipts_it = receipts.into_iter();
    for reg in 0..VM_REGISTER_COUNT {
        if let Receipt::Log { ra, .. } = receipts_it.next().expect("Missing receipt") {
            assert_eq!(ra, reg as u64);
        } else {
            unreachable!("Expected a log receipt");
        }
    }
}

#[test]
fn logd_memory_range_overflow() {
    let script = vec![
        op::not(0x30, RegId::ZERO),
        op::logd(RegId::ZERO, RegId::ZERO, 0x30, 1),
        op::ret(RegId::ONE),
    ];

    let receipts = run_script(script.into_iter().collect());
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[test]
fn logd_memory_out_of_range_fails() {
    let mut script = set_full_word(0x30, VM_MAX_RAM);
    script.push(op::logd(RegId::ZERO, RegId::ZERO, 0x30, 1));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[test]
fn logd_just_below_memory_limit_succeeds() {
    let mut script = set_full_word(0x30, VM_MAX_RAM - 100);
    script.push(op::movi(0x31, 100));
    script.push(op::aloc(0x31));
    script.push(op::logd(RegId::ZERO, RegId::ZERO, 0x30, 0x31));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());
    assert_success(&receipts);
}

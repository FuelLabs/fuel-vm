#![allow(dead_code)] // This module is used by other test, but clippy doesn't understand that

use fuel_asm::{op, Instruction};
use fuel_vm::prelude::*;

/// Set a register `r` to a Word-sized number value using left-shifts
pub fn set_full_word(r: RegisterId, v: Word) -> Vec<Instruction> {
    let r = u8::try_from(r).unwrap();
    let mut ops = vec![op::movi(r, 0)];
    for byte in v.to_be_bytes() {
        ops.push(op::ori(r, r, byte as Immediate12));
        ops.push(op::slli(r, r, 8));
    }
    ops.pop().unwrap(); // Remove last shift
    ops
}

/// Run a instructions-only script with reasonable defaults, and return receipts
pub fn run_script(script: Vec<Instruction>) -> Vec<Receipt> {
    let script = script.into_iter().collect();
    let mut client = MemoryClient::default();
    let tx = Transaction::script(0, 1_000_000, 0, script, vec![], vec![], vec![], vec![])
        .into_checked(0, &ConsensusParameters::DEFAULT, client.gas_costs())
        .expect("failed to generate a checked tx");
    client.transact(tx);
    client.receipts().expect("Expected receipts").as_ref().to_vec()
}

/// Assert that transaction didn't panic
pub fn assert_success(receipts: &[Receipt]) {
    if let Receipt::ScriptResult { result, .. } = receipts.last().unwrap() {
        if *result != ScriptExecutionResult::Success {
            panic!("Expected vm success, got {result:?} instead");
        }
    } else {
        unreachable!("No script result");
    }
}

/// Assert that transaction receipts end in a panic with the given reason
pub fn assert_panics(receipts: &[Receipt], reason: PanicReason) {
    if let Receipt::ScriptResult { result, .. } = receipts.last().unwrap() {
        if *result != ScriptExecutionResult::Panic {
            panic!("Expected vm panic, got {result:?} instead");
        }
    } else {
        unreachable!("No script result");
    }

    let n = receipts.len();
    assert!(n >= 2, "Invalid receipts len");
    if let Receipt::Panic { reason: pr, .. } = receipts.get(n - 2).unwrap() {
        assert_eq!(*pr.reason(), reason, "Panic reason differs for the expected reason");
    } else {
        unreachable!("No script receipt for a paniced tx");
    }
}

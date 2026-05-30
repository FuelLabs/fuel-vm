//! Differential tests: the Cranelift JIT must produce *bit-identical* observable
//! results to the pure interpreter. We run the same script with the JIT disabled and
//! enabled and assert the receipts (logged register values, gas used, panic reasons)
//! match exactly.

use alloc::{vec, vec::Vec};

use crate::{
    checked_transaction::IntoChecked,
    interpreter::{InterpreterParams, MemoryInstance},
    prelude::{Interpreter, MemoryStorage},
};
use fuel_asm::{Instruction, RegId, op};
use fuel_tx::{Finalizable, GasCosts, Receipt, Script, TransactionBuilder};

/// Run `ops` as a script and return the resulting receipts, with the JIT either on
/// or off. Uses real (non-free) gas costs so gas accounting parity is exercised.
fn run(ops: Vec<Instruction>, gas_limit: u64, jit: bool) -> Vec<Receipt> {
    let mut interp = Interpreter::<_, _, Script>::with_storage(
        MemoryInstance::new(),
        MemoryStorage::default(),
        InterpreterParams {
            gas_costs: GasCosts::default(),
            ..Default::default()
        },
    );
    interp.set_jit_enabled(jit);

    let tx = TransactionBuilder::script(ops.into_iter().collect(), vec![])
        .script_gas_limit(gas_limit)
        .max_fee_limit(0)
        .add_fee_input()
        .finalize();
    let ready = tx
        .into_checked_basic(Default::default(), &Default::default())
        .unwrap()
        .test_into_ready();

    interp.transact(ready).unwrap();
    interp.receipts().to_vec()
}

/// Assert JIT and interpreter agree for the given program.
fn assert_jit_matches(ops: Vec<Instruction>, gas_limit: u64) {
    let interp = run(ops.clone(), gas_limit, false);
    let jit = run(ops, gas_limit, true);
    assert_eq!(interp, jit, "JIT and interpreter receipts diverged");
}

#[test]
fn jit_is_actually_exercised() {
    // Prove the JIT path runs native code, not just that results match: a
    // straight-line ALU script must report a non-zero JIT instruction count.
    let r0 = 0x10;
    let mut ops = vec![op::movi(r0, 1)];
    for _ in 0..50 {
        ops.push(op::addi(r0, r0, 1));
    }
    ops.push(op::ret(RegId::ONE));

    let mut interp = Interpreter::<_, _, Script>::with_storage(
        MemoryInstance::new(),
        MemoryStorage::default(),
        InterpreterParams {
            gas_costs: GasCosts::default(),
            ..Default::default()
        },
    );
    interp.set_jit_enabled(true);
    let tx = TransactionBuilder::script(ops.into_iter().collect(), vec![])
        .script_gas_limit(1_000_000)
        .max_fee_limit(0)
        .add_fee_input()
        .finalize();
    let ready = tx
        .into_checked_basic(Default::default(), &Default::default())
        .unwrap()
        .test_into_ready();
    interp.transact(ready).unwrap();

    assert!(
        interp.jit_executed_instrs() >= 50,
        "expected the JIT to execute the ALU block natively, got {}",
        interp.jit_executed_instrs()
    );
}

/// Big mixed straight-line ALU block, logging the four working registers at the end.
fn mixed_alu_program() -> Vec<Instruction> {
    let (r0, r1, r2, r3) = (0x10, 0x11, 0x12, 0x13);
    let mut ops = vec![
        op::movi(r0, 0x0001_2345 & 0x3ffff),
        op::movi(r1, 0x000a_bcde & 0x3ffff),
        op::movi(r2, 0x0000_ffff),
        op::movi(r3, 0x0007_0f0f & 0x3ffff),
    ];
    for i in 0..200 {
        match i % 10 {
            0 => ops.push(op::addi(r0, r0, 7)),
            1 => ops.push(op::subi(r1, r1, 3)),
            2 => ops.push(op::xori(r2, r2, 0x5a)),
            3 => ops.push(op::ori(r3, r3, 0x0f)),
            4 => ops.push(op::andi(r0, r0, 0x0fff)),
            5 => ops.push(op::add(r1, r1, r2)),
            6 => ops.push(op::sub(r2, r2, r3)),
            7 => ops.push(op::slli(r3, r3, 1)),
            8 => ops.push(op::srli(r3, r3, 2)),
            _ => ops.push(op::move_(r0, r1)),
        }
        // Keep values bounded so `add`/`sub` don't overflow (which would bail, still
        // correct, but we want to exercise the inline fast path here).
        if i % 20 == 19 {
            ops.push(op::andi(r1, r1, 0x0fff));
            ops.push(op::andi(r2, r2, 0x0fff));
            ops.push(op::andi(r3, r3, 0x0fff));
        }
    }
    // LOG is not JIT-eligible: it terminates the block and surfaces the computed
    // register values into a receipt we can compare.
    ops.push(op::log(r0, r1, r2, r3));
    ops.push(op::ret(RegId::ONE));
    ops
}

#[test]
fn jit_matches_interpreter_mixed_alu() {
    assert_jit_matches(mixed_alu_program(), 1_000_000);
}

#[test]
fn jit_matches_interpreter_comparisons_and_shifts() {
    let (r0, r1, r2, r3) = (0x10, 0x11, 0x12, 0x13);
    let ops = vec![
        op::movi(r0, 100),
        op::movi(r1, 200),
        op::eq(r2, r0, r1),
        op::lt(r3, r0, r1),
        op::gt(r0, r0, r1),
        op::not(r1, r1),
        op::sll(r2, r1, r0), // shift by 0 (r0==0 now)
        op::srl(r3, r1, r1), // huge shift amount -> 0
        op::log(r0, r1, r2, r3),
        op::ret(RegId::ONE),
    ];
    assert_jit_matches(ops, 1_000_000);
}

#[test]
fn jit_matches_interpreter_on_overflow() {
    // ADD that overflows u64: JIT must bail and the interpreter must produce the same
    // ArithmeticOverflow panic (default flags = non-wrapping).
    let (r0, r1) = (0x10, 0x11);
    let mut ops = vec![op::not(r0, RegId::ZERO)]; // r0 = u64::MAX
    ops.push(op::movi(r1, 1));
    ops.push(op::add(r0, r0, r1)); // overflow
    ops.push(op::log(r0, r1, RegId::ZERO, RegId::ZERO));
    ops.push(op::ret(RegId::ONE));
    assert_jit_matches(ops, 1_000_000);
}

#[test]
fn jit_matches_interpreter_on_out_of_gas() {
    // Tight gas budget so OOG triggers mid-block: JIT must bail at the exact same
    // instruction the interpreter would, yielding identical gas/panic receipts.
    let r0 = 0x10;
    let mut ops = vec![op::movi(r0, 1)];
    for _ in 0..500 {
        ops.push(op::addi(r0, r0, 1));
    }
    ops.push(op::ret(RegId::ONE));
    // Small limit -> runs out partway through.
    assert_jit_matches(ops, 50);
}

#[test]
fn jit_matches_interpreter_reserved_register_dest() {
    // Writing a reserved register must error identically (the JIT declines such ops,
    // so the interpreter produces the ReservedRegisterNotWritable panic).
    let ops = vec![
        op::movi(0x10, 5),
        op::add(RegId::ZERO, 0x10, 0x10),
        op::ret(RegId::ONE),
    ];
    assert_jit_matches(ops, 1_000_000);
}

#[test]
fn jit_matches_interpreter_pc_relative() {
    // Regression: instructions reading $pc (reg 0x3) as a SOURCE operand mid-block —
    // e.g. Sway's PC-relative address arithmetic `ADD r, $pc, imm` — must observe the
    // current instruction's PC, not the block-start PC. The JIT previously held PC in an
    // SSA value and only flushed it at block exit, so these reads were stale (off by
    // 4*index). Found via a real o2 mainnet tx replay.
    let ops = vec![
        op::movi(0x10, 5),
        op::movi(0x11, 7),
        op::add(0x12, RegId::PC, 0x10),
        op::add(0x13, RegId::PC, 0x11),
        op::move_(0x14, RegId::PC),
        op::log(0x12, 0x13, 0x14, RegId::ZERO),
        op::ret(RegId::ONE),
    ];
    assert_jit_matches(ops, 1_000_000);
}

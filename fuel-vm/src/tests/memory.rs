#![cfg(feature = "std")]

use fuel_asm::PanicReason;
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};
use test_case::test_case;

use fuel_asm::{
    op,
    RegId,
};
use fuel_tx::Receipt;
use fuel_vm::{
    consts::VM_MAX_RAM,
    interpreter::InterpreterParams,
    prelude::*,
};

use super::test_helpers::{
    assert_panics,
    assert_success,
    run_script,
    set_full_word,
    RunResult,
};
use fuel_tx::ConsensusParameters;

fn setup(program: Vec<Instruction>) -> Transactor<MemoryInstance, MemoryStorage, Script> {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let height = Default::default();

    let consensus_params = ConsensusParameters::standard();

    let script = program.into_iter().collect();

    let tx = TransactionBuilder::script(script, vec![])
        .script_gas_limit(gas_limit)
        .maturity(maturity)
        .add_fee_input()
        .finalize()
        .into_checked(height, &consensus_params)
        .expect("failed to check tx");

    let interpreter_params = InterpreterParams::new(gas_price, &consensus_params);

    let mut vm = Transactor::new(MemoryInstance::new(), storage, interpreter_params);
    vm.transact(tx);
    vm
}

#[test]
fn test_lw() {
    let ops = vec![
        op::movi(0x10, 8),
        op::aloc(0x10),
        op::move_(0x10, RegId::HP),
        op::sw(0x10, RegId::ONE, 0),
        op::lw(0x13, 0x10, 0),
        op::ret(RegId::ONE),
    ];
    let vm = setup(ops);
    let vm: &Interpreter<_, MemoryStorage, Script> = vm.as_ref();
    let result = vm.registers()[0x13_usize];
    assert_eq!(1, result);
}

#[test]
fn test_lw_unaglined() {
    let ops = vec![
        op::movi(0x10, 9),
        op::aloc(0x10),
        op::move_(0x10, RegId::HP),
        op::sw(0x10, RegId::ONE, 0),
        op::lw(0x13, 0x10, 0),
        op::ret(RegId::ONE),
    ];
    let vm = setup(ops);
    let vm: &Interpreter<_, MemoryStorage, Script> = vm.as_ref();
    let result = vm.registers()[0x13_usize];
    assert_eq!(1, result);
}

#[test]
fn test_lb() {
    let ops = vec![
        op::movi(0x10, 8),
        op::aloc(0x10),
        op::move_(0x10, RegId::HP),
        op::sb(0x10, RegId::ONE, 0),
        op::lb(0x13, 0x10, 0),
        op::ret(RegId::ONE),
    ];
    let vm = setup(ops);
    let vm: &Interpreter<_, MemoryStorage, Script> = vm.as_ref();
    let result = vm.registers()[0x13_usize] as u8;
    assert_eq!(1, result);
}

#[test_case(op::sb, 0, 0, 0 => RunResult::Success([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sb, 1, 0, 0 => RunResult::Success([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sb, 1, 0, 1 => RunResult::Success([0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sb, 1, 1, 0 => RunResult::Success([0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sb, 1, 1, 1 => RunResult::Success([0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sb, u8::MAX as u64, 0, 0 => RunResult::Success([0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sb, 1, 15, 0 => RunResult::Success([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]))]
#[test_case(op::sb, 1, 0, 15 => RunResult::Success([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]))]
#[test_case(op::sb, 1, 10, 5 => RunResult::Success([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]))]
#[test_case(op::sb, 1, 16, 0 => RunResult::Panic(PanicReason::MemoryOverflow))]
#[test_case(op::sb, 1, 0, 16 => RunResult::Panic(PanicReason::MemoryOverflow))]
#[test_case(op::sw, u64::MAX, 0, 2 => RunResult::Panic(PanicReason::MemoryOverflow))]
#[test_case(op::sw, u64::MAX, 9, 0 => RunResult::Panic(PanicReason::MemoryOverflow))]
#[test_case(op::sw, u64::MAX, 1, 1 => RunResult::Panic(PanicReason::MemoryOverflow))]
#[test_case(op::sb, u64::MAX, 0, 0 => RunResult::Success([0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sqw, u64::MAX, 0, 0 => RunResult::Success([0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::shw, u64::MAX, 0, 0 => RunResult::Success([0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sw, u64::MAX, 0, 0 => RunResult::Success([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sb, u64::MAX, 1, 0 => RunResult::Success([0, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sqw, u64::MAX, 1, 0 => RunResult::Success([0, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::shw, u64::MAX, 1, 0 => RunResult::Success([0, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sw, u64::MAX, 1, 0 => RunResult::Success([0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sb, u64::MAX, 0, 1 => RunResult::Success([0, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sqw, u64::MAX, 0, 1 => RunResult::Success([0, 0, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::shw, u64::MAX, 0, 1 => RunResult::Success([0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0]))]
#[test_case(op::sw, u64::MAX, 0, 1 => RunResult::Success([0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]))]
fn test_store_ops(
    op: fn(RegId, RegId, u16) -> Instruction,
    value: Word,
    reg_offset: u16,
    imm_offset: u16,
) -> RunResult<[u8; 16]> {
    let reg_alloc_size = RegId::new(0x10);
    let reg_tmp_addr = RegId::new(0x11);
    let reg_value = RegId::new(0x12);

    let mut ops = set_full_word(reg_value.into(), value);
    ops.extend([
        // Allocate 16 bytes of heap
        op::movi(reg_alloc_size, 16),
        op::aloc(reg_alloc_size),
        // Run the operation targeting the heap
        op::addi(reg_tmp_addr, RegId::HP, reg_offset),
        op(reg_tmp_addr, reg_value, imm_offset),
        // Log the result and return
        op::logd(RegId::ZERO, RegId::ZERO, RegId::HP, reg_alloc_size),
        op::ret(RegId::ONE),
    ]);

    let receipts = run_script(ops);
    RunResult::extract(&receipts, |receipts| {
        let Some(Receipt::LogData { data, .. }) = receipts.first() else {
            return None;
        };
        let mut result = [0; 16];
        result.copy_from_slice(data.as_ref().unwrap());
        Some(result)
    })
}

#[test_case(op::lb, 0, 0 => RunResult::Success(0x00))]
#[test_case(op::lqw, 0, 0 => RunResult::Success(0x0001))]
#[test_case(op::lhw, 0, 0 => RunResult::Success(0x0001_0203))]
#[test_case(op::lw, 0, 0 => RunResult::Success(0x0001_0203_0405_0607))]
#[test_case(op::lb, 1, 0 => RunResult::Success(0x01))]
#[test_case(op::lqw, 1, 0 => RunResult::Success(0x0102))]
#[test_case(op::lhw, 1, 0 => RunResult::Success(0x0102_0304))]
#[test_case(op::lw, 1, 0 => RunResult::Success(0x0102_0304_0506_0708))]
#[test_case(op::lb, 0, 1 => RunResult::Success(0x01))]
#[test_case(op::lqw, 0, 1 => RunResult::Success(0x0203))]
#[test_case(op::lhw, 0, 1 => RunResult::Success(0x0405_0607))]
#[test_case(op::lw, 0, 1 => RunResult::Success(0x0809_0a0b_0c0d_0e0f))]
#[test_case(op::lb, 16, 0 => RunResult::Panic(PanicReason::MemoryOverflow))]
#[test_case(op::lb, 0, 16 => RunResult::Panic(PanicReason::MemoryOverflow))]
#[test_case(op::lqw, 15, 0 => RunResult::Panic(PanicReason::MemoryOverflow))]
#[test_case(op::lqw, 0, 8 => RunResult::Panic(PanicReason::MemoryOverflow))]
#[test_case(op::lhw, 13, 0 => RunResult::Panic(PanicReason::MemoryOverflow))]
#[test_case(op::lhw, 0, 4 => RunResult::Panic(PanicReason::MemoryOverflow))]
#[test_case(op::lw, 9, 0 => RunResult::Panic(PanicReason::MemoryOverflow))]
#[test_case(op::lw, 0, 2 => RunResult::Panic(PanicReason::MemoryOverflow))]
fn test_load_ops(
    op: fn(RegId, RegId, u16) -> Instruction,
    reg_offset: u16,
    imm_offset: u16,
) -> RunResult<Word> {
    let reg_alloc_size = RegId::new(0x10);
    let reg_tmp_addr = RegId::new(0x11);
    let reg_value = RegId::new(0x12);

    let mut ops = vec![
        // Allocate 16 bytes of heap
        op::movi(reg_alloc_size, 16),
        op::aloc(reg_alloc_size),
    ];

    // Intialize the heap with values 0x00, 0x01, 0x02, ...
    for i in 0..16 {
        ops.push(op::movi(reg_value, i as _));
        ops.push(op::sb(RegId::HP, reg_value, i));
    }

    ops.extend([
        // Run the operation targeting the heap
        op::addi(reg_tmp_addr, RegId::HP, reg_offset),
        op(reg_value, reg_tmp_addr, imm_offset),
        // Log the result and return
        op::log(reg_value, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    let receipts = run_script(ops);
    RunResult::extract(&receipts, |receipts| {
        let Some(Receipt::Log { ra, .. }) = receipts.first() else {
            return None;
        };
        Some(*ra)
    })
}

#[test]
fn test_aloc_sb_lb_last_byte_of_memory() {
    let ops = vec![
        op::move_(0x20, RegId::HP),
        op::movi(0x10, 1),
        op::aloc(0x10),
        op::move_(0x21, RegId::HP),
        op::sb(RegId::HP, 0x10, 0),
        op::lb(0x13, RegId::HP, 0),
        op::ret(RegId::ONE),
    ];
    let vm = setup(ops);
    let vm: &Interpreter<_, MemoryStorage, Script> = vm.as_ref();
    let r1 = vm.registers()[0x20_usize];
    let r2 = vm.registers()[0x21_usize];
    assert_eq!(r1 - 1, r2);
    let result = vm.registers()[0x13_usize] as u8;
    assert_eq!(1, result);
}

#[test_case(1, false)]
#[test_case(2, false)]
#[test_case(1, true)]
#[test_case(2, true)]
fn test_stack_and_heap_cannot_overlap(offset: u64, cause_error: bool) {
    // First, allocate almost all memory to heap, and then allocate the remaining
    // memory on the stack. If cause_error is set, then attempts to allocate one
    // byte too much here, causing a memory overflow error.

    let init_bytes = 12000; // Arbitrary number of bytes larger than SSP at start
    let mut ops = set_full_word(0x10, VM_MAX_RAM - init_bytes);
    ops.extend(&[
        op::aloc(0x10),
        op::movi(0x10, (init_bytes - offset).try_into().unwrap()),
        op::sub(0x10, 0x10, RegId::SP),
        op::aloc(0x10),
        op::cfei(
            (if cause_error { offset + 1 } else { offset })
                .try_into()
                .unwrap(),
        ),
        op::ret(RegId::ONE),
    ]);

    let vm = setup(ops);

    let mut receipts = vm.receipts().unwrap().to_vec();

    if cause_error {
        let _ = receipts.pop().unwrap(); // Script result unneeded, the panic receipt below is enough
        if let Receipt::Panic { reason, .. } = receipts.pop().unwrap() {
            assert!(matches!(reason.reason(), PanicReason::MemoryGrowthOverlap));
        } else {
            panic!("Expected tx panic when cause_error is set");
        }
    } else if let Receipt::ScriptResult { result, .. } = receipts.pop().unwrap() {
        assert!(matches!(result, ScriptExecutionResult::Success));
    } else {
        panic!("Expected tx success when cause_error is not set");
    }
}

/// tests for cfe & cfs
#[test]
fn dynamic_call_frame_ops() {
    const STACK_EXTEND_AMOUNT: u32 = 100u32;
    const STACK_SHRINK_AMOUNT: u32 = 50u32;
    let ops = vec![
        // log current stack pointer
        op::log(RegId::SP, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        // set stack extension amount for cfe into a register
        op::movi(0x10, STACK_EXTEND_AMOUNT),
        // extend stack dynamically
        op::cfe(0x10),
        // log the current stack pointer
        op::log(RegId::SP, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        // set stack shrink amount for cfs into a register
        op::movi(0x11, STACK_SHRINK_AMOUNT),
        // shrink the stack dynamically
        op::cfs(0x11),
        // return the current stack pointer
        op::ret(RegId::SP),
    ];

    let vm = setup(ops);

    let receipts = vm.receipts().unwrap().to_vec();
    // gather values of sp from the test
    let initial_sp = if let Receipt::Log { ra, .. } = receipts[0] {
        ra
    } else {
        panic!("expected receipt to be log")
    };
    let extended_sp = if let Receipt::Log { ra, .. } = receipts[1] {
        ra
    } else {
        panic!("expected receipt to be log")
    };
    let shrunken_sp = if let Receipt::Return { val, .. } = receipts[2] {
        val
    } else {
        panic!("expected receipt to be return")
    };

    // verify sp increased by expected amount
    assert_eq!(extended_sp, initial_sp + STACK_EXTEND_AMOUNT as u64);
    // verify sp decreased by expected amount
    assert_eq!(
        shrunken_sp,
        initial_sp + STACK_EXTEND_AMOUNT as u64 - STACK_SHRINK_AMOUNT as u64
    );
}

#[test]
fn dynamic_call_frame_ops_bug_missing_ssp_check() {
    let ops = vec![
        op::cfs(RegId::SP),
        op::slli(0x10, RegId::ONE, 26),
        op::aloc(0x10),
        op::sw(RegId::ZERO, 0x10, 0),
        op::ret(RegId::ONE),
    ];
    let receipts = run_script(ops);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[rstest::rstest]
fn test_mcl_and_mcli(
    #[values(0, 1, 7, 8, 9, 255, 256, 257)] count: u32,
    #[values(true, false)] half: bool, // Clear only first count/2 bytes
    #[values(true, false)] mcli: bool, // Test mcli instead of mcl
) {
    // Allocate count + 1 bytes of memory, so we can check that the last byte is not
    // cleared
    let mut ops = vec![op::movi(0x10, count + 1), op::aloc(0x10), op::movi(0x11, 1)];
    // Fill with ones
    for i in 0..(count + 1) {
        ops.push(op::sb(RegId::HP, 0x11, i as u16));
    }
    // Clear it, or only half if specified
    if mcli {
        if half {
            ops.push(op::mcli(RegId::HP, count / 2));
        } else {
            ops.push(op::mcli(RegId::HP, count));
        }
    } else {
        ops.push(op::movi(0x10, count));
        if half {
            ops.push(op::divi(0x10, 0x10, 2));
        }
        ops.push(op::mcl(RegId::HP, 0x10));
    }
    // Log the result and return
    ops.push(op::movi(0x10, count + 1));
    ops.push(op::logd(0, 0, RegId::HP, 0x10));
    ops.push(op::ret(RegId::ONE));

    let vm = setup(ops);
    let vm: &Interpreter<_, MemoryStorage, Script> = vm.as_ref();

    if let Some(Receipt::LogData { data, .. }) = vm.receipts().first() {
        let data = data.as_ref().unwrap();
        let c = count as usize;
        assert_eq!(data.len(), c + 1);
        if half {
            assert!(data[..c / 2] == vec![0u8; c / 2]);
            assert!(data[c / 2..] == vec![1u8; c - c / 2 + 1]);
        } else {
            assert!(data[..c] == vec![0u8; c]);
            assert!(data[c] == 1);
        }
    } else {
        panic!("Expected LogData receipt");
    }
}

#[rstest::rstest]
fn test_mcp_and_mcpi(
    #[values(0, 1, 7, 8, 9, 255, 256, 257)] count: u32,
    #[values(true, false)] mcpi: bool, // Test mcpi instead of mcp
) {
    // Allocate (count + 1) * 2 bytes of memory, so we can check that the last byte is not
    // copied
    let mut ops = vec![
        op::movi(0x10, (count + 1) * 2),
        op::aloc(0x10),
        op::movi(0x11, 1),
        op::movi(0x12, 2),
    ];
    // Fill count + 1 bytes with ones, and the next count + 1 bytes with twos
    for i in 0..(count + 1) * 2 {
        ops.push(op::sb(
            RegId::HP,
            if i < count + 1 { 0x11 } else { 0x12 },
            i as u16,
        ));
    }
    // Compute dst address
    ops.push(op::addi(0x11, RegId::HP, (count + 1) as u16));
    // Copy count bytes
    if mcpi {
        ops.push(op::mcpi(0x11, RegId::HP, count as u16));
    } else {
        ops.push(op::movi(0x10, count));
        ops.push(op::mcp(0x11, RegId::HP, 0x10));
    }
    // Log the result and return
    ops.push(op::movi(0x10, (count + 1) * 2));
    ops.push(op::logd(0, 0, RegId::HP, 0x10));
    ops.push(op::ret(RegId::ONE));

    let vm = setup(ops);
    let vm: &Interpreter<_, MemoryStorage, Script> = vm.as_ref();

    if let Some(Receipt::LogData { data, .. }) = vm.receipts().first() {
        let data = data.as_ref().unwrap();
        let c = count as usize;
        assert_eq!(data.len(), (c + 1) * 2);
        let mut expected = vec![1u8; c * 2 + 1];
        expected.push(2);
        assert!(data == &expected);
    } else {
        panic!("Expected LogData receipt");
    }
}

#[test]
fn test_meq() {
    let ops = vec![
        op::movi(0x20, 16),
        op::aloc(0x20),
        op::movi(0x30, 1234),
        op::movi(0x31, 1235),
        op::sw(RegId::HP, 0x30, 0),
        op::sw(RegId::HP, 0x31, 1),
        op::addi(0x32, RegId::HP, 8),
        op::meq(0x20, RegId::HP, RegId::HP, 8),
        op::meq(0x21, RegId::HP, 0x32, 8),
        op::log(0x20, 0x21, 0x22, 0x23),
        op::ret(RegId::ONE),
    ];
    let vm = setup(ops);
    let vm: &Interpreter<_, MemoryStorage, Script> = vm.as_ref();
    if let Some(Receipt::Log { ra, rb, rc, rd, .. }) = vm.receipts().first() {
        assert_eq!(*ra, 1);
        assert_eq!(*rb, 1);
        assert_eq!(*rc, 0);
        assert_eq!(*rd, 0);
    } else {
        panic!("Expected Log receipt");
    }
}

#[test]
fn test_push_pop_ops() {
    let rng = &mut StdRng::seed_from_u64(1234);
    for _ in 0..1000 {
        let mask = rng.gen::<u32>() & 0x00ff_ffff;
        let (push, pop) = if rng.gen() {
            (op::pshl(mask), op::popl(mask))
        } else {
            (op::pshh(mask), op::poph(mask))
        };
        let receipts = run_script(vec![
            op::log(RegId::WRITABLE, 0x20, 0x30, 0x3f),
            push,
            pop,
            op::log(RegId::WRITABLE, 0x20, 0x30, 0x3f),
            op::ret(RegId::ONE),
        ]);

        assert_success(&receipts);

        let Receipt::Log {
            ra: a1,
            rb: b1,
            rc: c1,
            rd: d1,
            ..
        } = receipts[0]
        else {
            panic!("Expected log receipt");
        };
        let Receipt::Log {
            ra: a2,
            rb: b2,
            rc: c2,
            rd: d2,
            ..
        } = receipts[1]
        else {
            panic!("Expected log receipt");
        };
        assert_eq!([a1, b1, c1, d1], [a2, b2, c2, d2]);
    }
}

#[test]
fn test_heap_not_executable() {
    let receipts = run_script(vec![
        op::movi(0x10, 16),
        op::aloc(0x10),
        op::sub(0x10, RegId::HP, RegId::IS),
        op::divi(0x10, 0x10, 4),
        op::jmp(0x10),
        op::ret(RegId::ONE),
    ]);

    if let Some(Receipt::Panic { reason, .. }) = receipts.first() {
        assert!(matches!(reason.reason(), PanicReason::MemoryNotExecutable));
    } else {
        panic!("Expected panic receipt");
    }
}

#[test]
fn test_shrunk_stack_remains_readable() {
    let nonce = 12345;
    let receipts = run_script(vec![
        op::movi(0x21, nonce),
        op::cfei(8),
        op::sw(RegId::SSP, 0x21, 0),
        op::cfsi(8),
        op::lw(0x20, RegId::SSP, 0),
        op::ret(0x20),
    ]);

    if let Some(Receipt::Return { val, .. }) = receipts.first() {
        assert_eq!(*val, nonce as u64);
    } else {
        panic!("Expected return receipt");
    }
}

#[test]
fn test_stack_extension_doesnt_zero_memory() {
    let canary = 12345;
    let receipts = run_script(vec![
        op::movi(0x21, canary),
        op::cfei(8),
        op::sw(RegId::SSP, 0x21, 0),
        op::cfsi(8),
        op::cfei(8),
        op::lw(0x20, RegId::SSP, 0),
        op::ret(0x20),
    ]);

    if let Some(Receipt::Return { val, .. }) = receipts.first() {
        assert_eq!(*val, canary as u64);
    } else {
        panic!("Expected return receipt");
    }
}

#[test]
fn test_shrunk_stack_is_not_writable() {
    let receipts = run_script(vec![
        op::movi(0x21, 12345),
        op::cfei(8),
        op::sw(RegId::SSP, 0x21, 0),
        op::cfsi(8),
        op::sw(RegId::SSP, 0x21, 0),
        op::ret(0x20),
    ]);

    if let Some(Receipt::Panic { reason, .. }) = receipts.first() {
        assert!(matches!(reason.reason(), PanicReason::MemoryOwnership));
    } else {
        panic!("Expected panic receipt");
    }
}

#[test]
fn test_heap_allocation_zeroes_memory() {
    let canary = 12345;
    let mut script = set_full_word(0x20, VM_MAX_RAM);
    script.extend(vec![
        // Extend stack to cover the whole memory
        op::sub(0x21, 0x20, RegId::SP),
        op::cfe(0x21),
        // Write a canary to the end of memory
        op::subi(0x21, RegId::SP, 8),
        op::movi(0x22, canary),
        op::sw(0x21, 0x22, 0),
        // Shrink stack
        op::cfsi(8),
        // Expand heap
        op::movi(0x23, 8),
        op::aloc(0x23),
        // Read the canary back to make sure the memory was zeroed
        op::lw(0x24, 0x21, 0),
        op::ret(0x24),
    ]);
    let receipts = run_script(script);
    if let Some(Receipt::Return { val, .. }) = receipts.first() {
        assert_eq!(*val, 0u64);
    } else {
        panic!("Expected return receipt");
    }
}

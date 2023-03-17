use test_case::test_case;

use fuel_asm::op;
use fuel_asm::RegId;
use fuel_tx::Receipt;
use fuel_vm::consts::VM_MAX_RAM;
use fuel_vm::prelude::*;

fn setup(program: Vec<Instruction>) -> Transactor<MemoryStorage, Script> {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    let script = program.into_iter().collect();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, &gas_costs)
        .expect("failed to check tx");

    let mut vm = Transactor::new(storage, Default::default(), Default::default());
    vm.transact(tx);
    vm
}

#[test]
fn test_lw() {
    let ops = vec![
        op::movi(0x10, 8),
        op::movi(0x11, 1),
        op::aloc(0x10),
        op::move_(0x10, RegId::HP),
        op::sw(0x10, 0x11, 0),
        op::lw(0x13, 0x10, 0),
        op::ret(RegId::ONE),
    ];
    let vm = setup(ops);
    let vm: &Interpreter<MemoryStorage, Script> = vm.as_ref();
    let result = vm.registers()[0x13_usize];
    assert_eq!(1, result);
}

#[test]
fn test_lb() {
    let ops = vec![
        op::movi(0x10, 8),
        op::movi(0x11, 1),
        op::aloc(0x10),
        op::move_(0x10, RegId::HP),
        op::sb(0x10, 0x11, 0),
        op::lb(0x13, 0x10, 0),
        op::ret(RegId::ONE),
    ];
    let vm = setup(ops);
    let vm: &Interpreter<MemoryStorage, Script> = vm.as_ref();
    let result = vm.registers()[0x13_usize] as u8;
    assert_eq!(1, result);
}

fn set_full_word(r: RegisterId, v: Word) -> Vec<Instruction> {
    let r = u8::try_from(r).unwrap();
    let mut ops = vec![op::movi(r, 0)];
    for byte in v.to_be_bytes() {
        ops.push(op::ori(r, r, byte as Immediate12));
        ops.push(op::slli(r, r, 8));
    }
    ops.pop().unwrap(); // Remove last shift
    ops
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
        op::cfei((if cause_error { offset } else { offset - 1 }).try_into().unwrap()),
        op::ret(RegId::ONE),
    ]);

    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    let script = ops.into_iter().collect();
    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, &gas_costs)
        .expect("failed to check tx");

    let mut vm = Transactor::new(storage, Default::default(), Default::default());
    vm.transact(tx);

    let mut receipts = vm.receipts().unwrap().to_vec();

    if cause_error {
        let _ = receipts.pop().unwrap(); // Script result unneeded, the panic receipt below is enough
        if let Receipt::Panic { reason, .. } = receipts.pop().unwrap() {
            assert!(matches!(reason.reason(), PanicReason::MemoryOverflow));
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

    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    let script = ops.into_iter().collect();
    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, &gas_costs)
        .expect("failed to check tx");

    let mut vm = Transactor::new(storage, Default::default(), Default::default());
    vm.transact(tx);

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

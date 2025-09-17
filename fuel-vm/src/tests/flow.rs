use alloc::{
    borrow::ToOwned,
    vec,
    vec::Vec,
};

use crate::{
    consts::*,
    prelude::*,
    script_with_data_offset,
    tests::test_helpers::assert_success,
    util::test_helpers::TestBuilder,
};
use fuel_asm::{
    Flags,
    RegId,
    op,
};
use fuel_crypto::Hasher;
use fuel_types::canonical::Serialize;
use itertools::Itertools;

const SET_STATUS_REG: u8 = 0x29;

#[test]
fn can_execute_empty_script_transaction() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    let empty_script = vec![];

    let result = test_context
        .start_script(empty_script, vec![])
        .script_gas_limit(gas_limit)
        .fee_input()
        .execute();

    let receipts = result.receipts();

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Return { val: 1, .. }));
    assert!(matches!(
        receipts[1],
        Receipt::ScriptResult {
            result: ScriptExecutionResult::Success,
            ..
        }
    ));
}

#[test]
fn code_copy() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    let program_ops = vec![
        op::movi(0x10, 0x11),
        op::movi(0x11, 0x2a),
        op::add(0x12, 0x10, 0x11),
        op::log(0x10, 0x11, 0x12, 0x00),
        op::ret(0x20),
    ];

    let program = program_ops.clone().into_iter().collect::<Vec<u8>>();
    let contract_size = program.len();
    let contract_id = test_context
        .setup_contract(program_ops, None, None)
        .contract_id;

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, 2048),
            op::aloc(0x10),
            op::move_(0x10, RegId::HP),
            op::movi(0x20, data_offset as Immediate18),
            op::add(0x11, RegId::ZERO, 0x20),
            op::movi(0x12, contract_size as Immediate18),
            op::ccp(0x10, 0x11, RegId::ZERO, 0x12),
            op::addi(0x21, 0x20, ContractId::LEN as Immediate12),
            op::meq(0x30, 0x21, 0x10, 0x12),
            op::ret(0x30),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let mut script_data = contract_id.to_vec();
    script_data.extend(program.as_slice());

    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let receipts = result.receipts();
    let ret = receipts
        .first()
        .expect("A `RET` opcode was part of the program.");

    assert_eq!(1, ret.val().expect("A constant `1` was returned."));
}

#[test]
fn call() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    let program = vec![
        op::movi(0x10, 0x11),
        op::movi(0x11, 0x2a),
        op::add(0x12, 0x10, 0x11),
        op::log(0x10, 0x11, 0x12, 0x00),
        op::ret(0x12),
    ];

    let contract_id = test_context.setup_contract(program, None, None).contract_id;

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::addi(0x11, 0x10, ContractId::LEN as Immediate12),
            op::call(0x10, RegId::ZERO, 0x10, 0x10),
            op::ret(0x30),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let mut script_data = contract_id.to_vec();
    script_data.extend([0u8; WORD_SIZE * 2]);

    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let receipts = result.receipts();

    assert_eq!(receipts[0].id(), None);
    assert_eq!(
        receipts[0].to().expect("Receipt value failed").to_owned(),
        contract_id
    );
    assert_eq!(receipts[1].ra().expect("Receipt value failed"), 0x11);
    assert_eq!(receipts[1].rb().expect("Receipt value failed"), 0x2a);
    assert_eq!(receipts[1].rc().expect("Receipt value failed"), 0x3b);
}

#[test]
fn call_frame_code_offset() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    let program_ops = vec![
        op::log(RegId::PC, RegId::FP, RegId::SSP, RegId::SP),
        op::noop(),
        op::noop(),
        op::noop(),
        op::noop(),
        op::movi(0x10, 1),
        op::ret(RegId::ONE),
    ];

    let program = program_ops.clone().into_iter().collect::<Vec<u8>>();
    let contract_id = test_context
        .setup_contract(program_ops, None, None)
        .contract_id;

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::log(RegId::SP, 0, 0, 0),
            op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let mut script_data = vec![];

    script_data.extend(contract_id.as_ref());
    script_data.extend(Word::default().to_be_bytes());
    script_data.extend(Word::default().to_be_bytes());

    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let asset_id = AssetId::default();

    let frame = CallFrame::new(
        contract_id,
        asset_id,
        [0; VM_REGISTER_COUNT],
        program.len(),
        0,
        0,
    )
    .unwrap();
    let stack = (frame.to_bytes().len() + frame.code_size_padded()) as Word;

    let receipts = result.receipts();

    let sp = receipts[0].ra().expect("Expected $ra from receipt");
    let fp = receipts[2].rb().expect("Expected $rb from receipt");
    let ssp = receipts[2].rc().expect("Expected $rc from receipt");
    let sp_p = receipts[2].rd().expect("Expected $rd from receipt");

    assert_eq!(ssp, sp + stack);
    assert_eq!(ssp, fp + stack);
    assert_eq!(ssp, sp_p);
}

#[test]
fn call_zeroes_flag() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    let program = vec![
        op::log(RegId::FLAG, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ];

    let contract_id = test_context.setup_contract(program, None, None).contract_id;

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, Flags::UNSAFEMATH.bits().try_into().unwrap()),
            op::flag(0x10),
            op::movi(0x10, data_offset as Immediate18),
            op::addi(0x11, 0x10, ContractId::LEN as Immediate12),
            op::call(0x10, RegId::ZERO, 0x10, 0x10),
            op::log(RegId::FLAG, RegId::ZERO, RegId::ZERO, RegId::ZERO),
            op::ret(0x30),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let mut script_data = contract_id.to_vec();
    script_data.extend([0u8; WORD_SIZE * 2]);

    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let receipts = result.receipts();

    assert_success(receipts);

    let Receipt::Log {
        ra: flag_in_call, ..
    } = &receipts[1]
    else {
        panic!("Expected log receipt");
    };
    let Receipt::Log {
        ra: flag_after_call,
        ..
    } = &receipts[3]
    else {
        panic!("Expected log receipt");
    };

    assert_eq!(
        *flag_in_call,
        Flags::empty().bits(),
        "Call should zero $flag"
    );
    assert_eq!(
        *flag_after_call,
        Flags::UNSAFEMATH.bits(),
        "Return should restore $flag"
    );
}

#[test]
fn revert_from_call_immediately_ends_execution() {
    let mut test_context = TestBuilder::new(2322u64);
    // call a contract that reverts and then verify the revert is only logged once
    let gas_limit = 1_000_000;

    // setup a contract which immediately reverts
    let contract_id = test_context
        .setup_contract(vec![op::rvrt(RegId::ONE)], None, None)
        .contract_id;

    // setup a script to call the contract
    let (script_ops, _) = script_with_data_offset!(
        data_offset,
        vec![
            // load call data to 0x10
            op::movi(0x10, data_offset),
            op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );
    let script_data: Vec<u8> = [Call::new(contract_id, 0, 0).to_bytes().as_slice()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // initiate the call to the contract which reverts
    let result = test_context
        .start_script(script_ops, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    // verify there is only 1 revert receipt
    let revert_receipts = result
        .receipts()
        .as_ref()
        .iter()
        .filter(|r| matches!(r, Receipt::Revert { .. }))
        .collect_vec();

    assert_eq!(revert_receipts.len(), 1);
}

/// Makes sure that infinte recursion with CALL instruction doesn't crash
#[test]
fn repeated_nested_calls() {
    let gas_limit = 1_000_000;
    let mut test_context = TestBuilder::new(2322u64);

    // setup a contract which immediately reverts
    let contract_id = test_context
        .setup_contract(
            vec![
                op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
                op::ret(RegId::ONE),
            ],
            None,
            None,
        )
        .contract_id;

    let script = vec![
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
        op::ret(RegId::ONE),
    ];
    let script_data = Call::new(contract_id, 0, 1000).to_bytes();

    // initiate the call to the contract which reverts
    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();
    let receipts = result.receipts();

    if let Receipt::ScriptResult { result, .. } = receipts[receipts.len() - 1] {
        if result != ScriptExecutionResult::Panic {
            panic!("Expected vm panic, got {result:?} instead");
        }
    } else {
        unreachable!("No script result");
    }

    if let Receipt::Panic { reason: pr, .. } = receipts[receipts.len() - 2] {
        assert_eq!(
            *pr.reason(),
            PanicReason::OutOfGas,
            "Panic reason differs for the expected reason"
        );
    } else {
        unreachable!("No script receipt for a paniced tx");
    }
}

#[test]
fn revert() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;
    #[rustfmt::skip]
    let call_arguments_parser = vec![
        op::addi(0x10, RegId::FP, CallFrame::a_offset() as Immediate12),
        op::lw(0x10, 0x10, 0),
        op::addi(0x11, RegId::FP, CallFrame::b_offset() as Immediate12),
        op::lw(0x11, 0x11, 0),
    ];

    #[rustfmt::skip]
    let routine_add_word_to_state = vec![
        op::jnei(0x10, 0x30, 13),            // (0, b) Add word to state
        op::lw(0x20, 0x11, 4),               // r[0x20]      := m[b+32, 8]
        op::srw(0x21, SET_STATUS_REG, 0x11), // r[0x21]      := s[m[b, 32], 8]
        op::add(0x20, 0x20, 0x21),           // r[0x20]      += r[0x21]
        op::sww(0x11, SET_STATUS_REG, 0x20), // s[m[b,32]]   := r[0x20]
        op::log(0x20, 0x21, 0x00, 0x00),
        op::ret(RegId::ONE),
    ];

    let program = call_arguments_parser
        .into_iter()
        .chain(routine_add_word_to_state)
        .collect_vec();

    let contract_id = test_context.setup_contract(program, None, None).contract_id;

    let (script, data_offset) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let mut script_data = vec![];

    // Routine to be called: Add word to state
    let routine: Word = 0;

    // Offset of the script data relative to the call data
    let call_data_offset = data_offset as usize + ContractId::LEN + 2 * WORD_SIZE;
    let call_data_offset = call_data_offset as Word;

    // Key and value to be added
    let key = Hasher::hash(b"some key");
    let val: Word = 150;

    // Script data containing the call arguments (contract, a, b) and (key, value)
    script_data.extend(contract_id.as_ref());
    script_data.extend(routine.to_be_bytes());
    script_data.extend(call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(val.to_be_bytes());

    // initiate the call to the contract which reverts
    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();
    let receipts = result.receipts();
    let state = test_context
        .get_storage()
        .contract_state(&contract_id, &key);

    // Assert the state of `key` is mutated to `val`
    assert_eq!(
        &val.to_be_bytes()[..],
        &state.as_ref().as_ref()[..WORD_SIZE]
    );

    // Expect the correct receipt
    assert_eq!(receipts[1].ra().expect("Register value expected"), val);
    assert_eq!(receipts[1].rb().expect("Register value expected"), 0);

    // Create a script with revert instruction
    let script = vec![
        op::movi(0x10, data_offset),
        op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
        op::rvrt(RegId::ONE),
    ];

    let mut script_data = vec![];

    // Value to be added
    let rev: Word = 250;

    // Script data containing the call arguments (contract, a, b) and (key, value)
    script_data.extend(contract_id.as_ref());
    script_data.extend(routine.to_be_bytes());
    script_data.extend(call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(rev.to_be_bytes());

    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();
    let state = test_context
        .get_storage()
        .contract_state(&contract_id, &key);

    assert_eq!(
        &val.to_be_bytes()[..],
        &state.as_ref().as_ref()[..WORD_SIZE]
    );

    // Expect the correct receipt
    let receipts = result.receipts();

    assert_eq!(
        receipts[1].ra().expect("Register value expected"),
        val + rev
    );
    assert_eq!(receipts[1].rb().expect("Register value expected"), val);

    match receipts[3] {
        Receipt::Revert { ra: 1, .. } => (),
        _ => panic!("Expected revert receipt: {:?}", receipts[3]),
    }
}

#[test]
fn retd_from_top_of_heap() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    const REG_SIZE: u8 = RegId::WRITABLE.to_u8();
    const REG_PTR: u8 = RegId::WRITABLE.to_u8() + 1;

    #[rustfmt::skip]
    let script = vec![
        op::movi(REG_SIZE, 32),              // Allocate 32 bytes.
        op::aloc(REG_SIZE),                  // $hp -= 32.
        op::move_(REG_PTR, RegId::HP),       // Pointer is $hp, first byte in allocated buffer.
        op::retd(REG_PTR, REG_SIZE),         // Return the allocated buffer.
    ];

    let result = test_context
        .start_script(script, vec![])
        .gas_price(0)
        .script_gas_limit(gas_limit)
        .fee_input()
        .execute();

    let receipts = result.receipts();

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::ReturnData { .. }));
}

#[test]
fn logd_from_top_of_heap() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    const REG_SIZE: RegId = RegId::WRITABLE;
    let reg_ptr: RegId = RegId::new(u8::from(RegId::WRITABLE) + 1);

    #[rustfmt::skip]
    let script = vec![
        op::movi(REG_SIZE, 32),                                               // Allocate 32 bytes.
        op::aloc(REG_SIZE),                                                   // $hp -= 32.
        op::move_(reg_ptr, RegId::HP),                                        // Pointer is $hp, first byte in allocated buffer.
        op::logd(RegId::ZERO, RegId::ZERO, reg_ptr, REG_SIZE), // Log the whole buffer
        op::ret(RegId::ONE),                                                     // Return
    ];

    let result = test_context
        .start_script(script, vec![])
        .gas_price(0)
        .script_gas_limit(gas_limit)
        .fee_input()
        .execute();

    let receipts = result.receipts();

    // Expect the correct receipt
    assert_eq!(receipts.len(), 3);
    assert!(matches!(receipts[0], Receipt::LogData { .. }));
}

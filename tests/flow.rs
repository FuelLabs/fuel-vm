#![allow(clippy::iter_cloned_collect)] // https://github.com/rust-lang/rust-clippy/issues/9119

use fuel_crypto::Hasher;
use fuel_types::bytes;
use fuel_vm::{consts::*, prelude::*, script_with_data_offset, util::test_helpers::TestBuilder};
use itertools::Itertools;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[test]
fn code_copy() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    let salt: Salt = rng.gen();

    let program: Vec<u8> = vec![
        Opcode::MOVI(0x10, 0x11),
        Opcode::MOVI(0x11, 0x2a),
        Opcode::ADD(0x12, 0x10, 0x11),
        Opcode::LOG(0x10, 0x11, 0x12, 0x00),
        Opcode::RET(0x20),
    ]
    .iter()
    .copied()
    .collect();
    let program = Witness::from(program.as_slice());

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract = contract.id(&salt, &contract_root, &state_root);

    let contract_size = program.as_ref().len();
    let output = Output::contract_created(contract, state_root);

    // Deploy the contract
    let tx = Transaction::create(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        0,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program.clone()],
    );

    client.transact(tx);

    let mut script_ops = vec![
        Opcode::MOVI(0x10, 2048),
        Opcode::ALOC(0x10),
        Opcode::ADDI(0x10, REG_HP, 0x01),
        Opcode::MOVI(0x20, 0x00),
        Opcode::ADD(0x11, REG_ZERO, 0x20),
        Opcode::MOVI(0x12, contract_size as Immediate18),
        Opcode::CCP(0x10, 0x11, REG_ZERO, 0x12),
        Opcode::ADDI(0x21, 0x20, ContractId::LEN as Immediate12),
        Opcode::MEQ(0x30, 0x21, 0x10, 0x12),
        Opcode::RET(0x30),
    ];

    let script = script_ops.iter().copied().collect();
    let mut script_data = contract.to_vec();
    script_data.extend(program.as_ref());
    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), contract);
    let output = Output::contract(0, rng.gen(), rng.gen());

    let mut tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    );

    let script_data_mem = client.tx_offset() + tx.script_data_offset().unwrap();
    script_ops[3] = Opcode::MOVI(0x20, script_data_mem as Immediate18);
    let script_mem: Vec<u8> = script_ops.iter().copied().collect();

    match &mut tx {
        Transaction::Script { script, .. } => script.as_mut_slice().copy_from_slice(script_mem.as_slice()),
        _ => unreachable!(),
    }

    let receipts = client.transact(tx);
    let ret = receipts.first().expect("A `RET` opcode was part of the program.");

    assert_eq!(1, ret.val().expect("A constant `1` was returned."));
}

#[test]
fn call() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;
    let salt: Salt = rng.gen();

    let program: Vec<u8> = vec![
        Opcode::MOVI(0x10, 0x11),
        Opcode::MOVI(0x11, 0x2a),
        Opcode::ADD(0x12, 0x10, 0x11),
        Opcode::LOG(0x10, 0x11, 0x12, 0x00),
        Opcode::RET(0x12),
    ]
    .iter()
    .copied()
    .collect();
    let program = Witness::from(program.as_slice());

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract = contract.id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract, state_root);

    // Deploy the contract
    let tx = Transaction::create(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        0,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program],
    );

    assert!(Transactor::new(&mut storage, Default::default())
        .transact(tx)
        .is_success());

    let mut script_ops = vec![
        Opcode::MOVI(0x10, 0x00),
        Opcode::ADDI(0x11, 0x10, ContractId::LEN as Immediate12),
        Opcode::CALL(0x10, REG_ZERO, 0x10, 0x10),
        Opcode::RET(0x30),
    ];

    let script = script_ops.iter().copied().collect();
    let mut script_data = contract.to_vec();
    script_data.extend(&[0u8; WORD_SIZE * 2]);
    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), contract);
    let output = Output::contract(0, rng.gen(), rng.gen());

    let mut tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    );

    let params = ConsensusParameters::default();

    let script_data_mem = params.tx_offset() + tx.script_data_offset().unwrap();
    script_ops[0] = Opcode::MOVI(0x10, script_data_mem as Immediate18);
    let script_mem: Vec<u8> = script_ops.iter().copied().collect();

    match &mut tx {
        Transaction::Script { script, .. } => script.as_mut_slice().copy_from_slice(script_mem.as_slice()),
        _ => unreachable!(),
    }

    let receipts = Transactor::new(&mut storage, params)
        .transact(tx)
        .receipts()
        .expect("Failed to execute script")
        .to_owned();

    assert_eq!(receipts[1].ra().expect("Receipt value failed"), 0x11);
    assert_eq!(receipts[1].rb().expect("Receipt value failed"), 0x2a);
    assert_eq!(receipts[1].rc().expect("Receipt value failed"), 0x3b);
}

#[test]
fn call_frame_code_offset() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    let salt: Salt = rng.gen();
    let bytecode_witness_index = 0;
    let program: Vec<u8> = vec![
        Opcode::LOG(REG_PC, REG_FP, REG_SSP, REG_SP),
        Opcode::NOOP,
        Opcode::NOOP,
        Opcode::NOOP,
        Opcode::NOOP,
        Opcode::MOVI(0x10, 1),
        Opcode::RET(REG_ONE),
    ]
    .into_iter()
    .collect();

    let contract = Contract::from(program.as_slice());
    let root = contract.root();
    let state_root = Contract::default_state_root();
    let id = contract.id(&salt, &root, &state_root);

    let input = Input::coin_signed(rng.gen(), rng.gen(), 0, rng.gen(), 0, maturity);
    let output = Output::contract_created(id, state_root);

    let deploy = Transaction::create(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        bytecode_witness_index,
        salt,
        vec![],
        vec![input],
        vec![output],
        vec![program.clone().into()],
    );

    assert!(Transactor::new(&mut storage, Default::default())
        .transact(deploy)
        .is_success());

    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), id);
    let output = Output::contract(0, rng.gen(), rng.gen());

    let script_len = 16;

    let params = ConsensusParameters::default();

    // Based on the defined script length, we set the appropriate data offset
    let script_data_offset = params.tx_offset() + Transaction::script_offset() + script_len;
    let script_data_offset = script_data_offset as Immediate18;

    let script = vec![
        Opcode::MOVI(0x10, script_data_offset),
        Opcode::LOG(REG_SP, 0, 0, 0),
        Opcode::CALL(0x10, REG_ZERO, 0x10, REG_CGAS),
        Opcode::RET(REG_ONE),
    ]
    .iter()
    .copied()
    .collect::<Vec<u8>>();

    let mut script_data = vec![];

    script_data.extend(id.as_ref());
    script_data.extend(&Word::default().to_be_bytes());
    script_data.extend(&Word::default().to_be_bytes());

    let script = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    );

    let mut vm = Interpreter::with_storage(storage, params);

    vm.transact(script).expect("Failed to call deployed contract");

    let asset_id = AssetId::default();
    let contract = Contract::from(program.as_ref());

    let mut frame = CallFrame::new(id, asset_id, [0; VM_REGISTER_COUNT], 0, 0, contract);
    let stack = frame.to_bytes().len() as Word;

    let receipts = vm.receipts();

    let sp = receipts[0].ra().expect("Expected $ra from receipt");
    let fp = receipts[2].rb().expect("Expected $rb from receipt");
    let ssp = receipts[2].rc().expect("Expected $rc from receipt");
    let sp_p = receipts[2].rd().expect("Expected $rd from receipt");

    assert_eq!(ssp, sp + stack);
    assert_eq!(ssp, fp + stack);
    assert_eq!(ssp, sp_p);
}

#[test]
fn revert_from_call_immediately_ends_execution() {
    // call a contract that reverts and then verify the revert is only logged once
    let gas_limit = 1_000_000;

    let mut test_context = TestBuilder::new(2322u64);

    // setup a contract which immediately reverts
    let contract_id = test_context
        .setup_contract(vec![Opcode::RVRT(REG_ONE)], None, None)
        .contract_id;

    // setup a script to call the contract
    let (script_ops, _) = script_with_data_offset!(
        data_offset,
        vec![
            // load call data to 0x10
            Opcode::MOVI(0x10, data_offset),
            Opcode::CALL(0x10, REG_ZERO, REG_ZERO, REG_CGAS),
            Opcode::RET(REG_ONE),
        ],
        test_context.tx_offset()
    );
    let script_data: Vec<u8> = [Call::new(contract_id, 0, 0).to_bytes().as_slice()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // initiate the call to the contract which reverts
    let result = test_context
        .gas_limit(gas_limit)
        .contract_input(contract_id)
        .contract_output(&contract_id)
        .script(script_ops)
        .script_data(script_data)
        .execute();

    // verify there is only 1 revert receipt
    let revert_receipts = result
        .receipts()
        .iter()
        .filter(|r| matches!(r, Receipt::Revert { .. }))
        .collect_vec();

    assert_eq!(revert_receipts.len(), 1);
}

#[test]
fn jump_if_not_zero_immediate_jump() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    #[rustfmt::skip]
    let script_jnzi_does_jump = vec![
        Opcode::JNZI(REG_ONE, 2),   // Jump to last instr if reg one is zero
        Opcode::RVRT(REG_ONE),       // Revert
        Opcode::RET(REG_ONE),        // Return successfully
    ].iter()
    .copied()
    .collect::<Vec<u8>>();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script_jnzi_does_jump,
        vec![],
        vec![],
        vec![],
        vec![],
    );

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Return { .. }));
}

#[test]
fn jump_if_not_zero_immediate_no_jump() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    #[rustfmt::skip]
    let script_jnzi_does_not_jump = vec![
        Opcode::JNZI(REG_ZERO, 2),   // Jump to last instr if reg zero is zero
        Opcode::RVRT(REG_ONE),       // Revert
        Opcode::RET(REG_ONE),        // Return successfully
    ].iter()
    .copied()
    .collect::<Vec<u8>>();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script_jnzi_does_not_jump,
        vec![],
        vec![],
        vec![],
        vec![],
    );

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Revert { .. }));
}

#[test]
fn jump_dynamic() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    #[rustfmt::skip]
    let script = vec![
        Opcode::MOVI(REG_WRITABLE, 3),  // Jump target: last instr
        Opcode::JMP(REG_WRITABLE),      // Jump
        Opcode::RVRT(REG_ONE),          // Revert
        Opcode::RET(REG_ONE),           // Return successfully
    ].iter()
    .copied()
    .collect::<Vec<u8>>();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        vec![],
        vec![],
        vec![],
        vec![],
    );

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Return { .. }));
}

#[test]
fn jump_dynamic_condition_true() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    #[rustfmt::skip]
    let script = vec![
        Opcode::MOVI(REG_WRITABLE, 3),                  // Jump target: last instr
        Opcode::JNE(REG_ZERO, REG_ONE, REG_WRITABLE),   // Conditional jump (yes, because 0 != 1)
        Opcode::RVRT(REG_ONE),                          // Revert
        Opcode::RET(REG_ONE),                           // Return successfully
    ].iter()
    .copied()
    .collect::<Vec<u8>>();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        vec![],
        vec![],
        vec![],
        vec![],
    );

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Return { .. }));
}

#[test]
fn jump_dynamic_condition_false() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    #[rustfmt::skip]
    let script = vec![
        Opcode::MOVI(REG_WRITABLE, 3),                  // Jump target: last instr
        Opcode::JNE(REG_ZERO, REG_ZERO, REG_WRITABLE),  // Conditional jump (no, because 0 != 0)
        Opcode::RVRT(REG_ONE),                          // Revert
        Opcode::RET(REG_ONE),                           // Return successfully
    ].iter()
    .copied()
    .collect::<Vec<u8>>();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        vec![],
        vec![],
        vec![],
        vec![],
    );

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Revert { .. }));
}

#[test]
fn revert() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    let salt: Salt = rng.gen();

    #[rustfmt::skip]
    let call_arguments_parser: Vec<Opcode> = vec![
        Opcode::ADDI(0x10, REG_FP, CallFrame::a_offset() as Immediate12),
        Opcode::LW(0x10, 0x10, 0),
        Opcode::ADDI(0x11, REG_FP, CallFrame::b_offset() as Immediate12),
        Opcode::LW(0x11, 0x11, 0),
    ];

    #[rustfmt::skip]
    let routine_add_word_to_state: Vec<Opcode> = vec![
        Opcode::JNEI(0x10, 0x30, 13),       // (0, b) Add word to state
        Opcode::LW(0x20, 0x11, 4),          // r[0x20]      := m[b+32, 8]
        Opcode::SRW(0x21, 0x11),            // r[0x21]      := s[m[b, 32], 8]
        Opcode::ADD(0x20, 0x20, 0x21),      // r[0x20]      += r[0x21]
        Opcode::SWW(0x11, 0x20),            // s[m[b,32]]   := r[0x20]
        Opcode::LOG(0x20, 0x21, 0x00, 0x00),
        Opcode::RET(REG_ONE),
    ];

    let program: Witness = call_arguments_parser
        .into_iter()
        .chain(routine_add_word_to_state.into_iter())
        .collect::<Vec<u8>>()
        .into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract = contract.id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract, state_root);

    let bytecode_witness = 0;
    let tx = Transaction::create(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        bytecode_witness,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program],
    );

    // Deploy the contract into the blockchain
    client.transact(tx);

    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), contract);
    let output = Output::contract(0, rng.gen(), rng.gen());

    // The script needs to locate the data offset at runtime. Hence, we need to know
    // upfront the serialized size of the script so we can set the registers
    // accordingly.
    //
    // This variable is created to assert we have correct script size in the
    // instructions.
    let script_len = 16;

    // Based on the defined script length, we set the appropriate data offset
    let script_data_offset = client.tx_offset() + Transaction::script_offset() + script_len;
    let script_data_offset = script_data_offset as Immediate18;

    let script = vec![
        Opcode::MOVI(0x10, script_data_offset),
        Opcode::CALL(0x10, REG_ZERO, REG_ZERO, REG_CGAS),
        Opcode::RET(REG_ONE),
    ]
    .iter()
    .copied()
    .collect::<Vec<u8>>();

    // Assert the offsets are set correctnly
    let offset = client.tx_offset() + Transaction::script_offset() + bytes::padded_len(script.as_slice());
    assert_eq!(script_data_offset, offset as Immediate18);

    let mut script_data = vec![];

    // Routine to be called: Add word to state
    let routine: Word = 0;

    // Offset of the script data relative to the call data
    let call_data_offset = script_data_offset as usize + ContractId::LEN + 2 * WORD_SIZE;
    let call_data_offset = call_data_offset as Word;

    // Key and value to be added
    let key = Hasher::hash(b"some key");
    let val: Word = 150;

    // Script data containing the call arguments (contract, a, b) and (key, value)
    script_data.extend(contract.as_ref());
    script_data.extend(&routine.to_be_bytes());
    script_data.extend(&call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(&val.to_be_bytes());

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    );

    // Assert the initial state of `key` is empty
    let state = client.as_ref().contract_state(&contract, &key);
    assert_eq!(Bytes32::default(), state.into_owned());

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");
    let state = client.as_ref().contract_state(&contract, &key);

    // Assert the state of `key` is mutated to `val`
    assert_eq!(&val.to_be_bytes()[..], &state.as_ref()[..WORD_SIZE]);

    // Expect the correct receipt
    assert_eq!(receipts[1].ra().expect("Register value expected"), val);
    assert_eq!(receipts[1].rb().expect("Register value expected"), 0);

    // Create a script with revert instruction
    let script = vec![
        Opcode::MOVI(0x10, script_data_offset),
        Opcode::CALL(0x10, REG_ZERO, 0x10, REG_CGAS),
        Opcode::RVRT(REG_ONE),
    ]
    .iter()
    .copied()
    .collect::<Vec<u8>>();

    let mut script_data = vec![];

    // Value to be added
    let rev: Word = 250;

    // Script data containing the call arguments (contract, a, b) and (key, value)
    script_data.extend(contract.as_ref());
    script_data.extend(&routine.to_be_bytes());
    script_data.extend(&call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(&rev.to_be_bytes());

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    );

    // Assert the state of `key` is reverted to `val`
    let state = client.as_ref().contract_state(&contract, &key);

    assert_eq!(&val.to_be_bytes()[..], &state.as_ref()[..WORD_SIZE]);

    // Expect the correct receipt
    let receipts = client.transact(tx);

    assert_eq!(receipts[1].ra().expect("Register value expected"), val + rev);
    assert_eq!(receipts[1].rb().expect("Register value expected"), val);

    match receipts[3] {
        Receipt::Revert { ra, .. } if ra == 1 => (),
        _ => panic!("Expected revert receipt: {:?}", receipts[3]),
    }
}

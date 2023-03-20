use fuel_asm::{op, RegId};
use fuel_crypto::Hasher;
use fuel_tx::{
    field::{Script as ScriptField, ScriptData},
    Script,
};
use fuel_types::bytes;
use fuel_vm::{consts::*, prelude::*, script_with_data_offset, util::test_helpers::TestBuilder};
use itertools::Itertools;
use rand::{rngs::StdRng, Rng, SeedableRng};

const SET_STATUS_REG: u8 = 0x29;

#[test]
fn can_execute_empty_script_transaction() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    let empty_script = vec![];

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        empty_script,
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate a checked tx");

    let receipts = client.transact(tx);

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
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    let salt: Salt = rng.gen();

    let program: Vec<u8> = vec![
        op::movi(0x10, 0x11),
        op::movi(0x11, 0x2a),
        op::add(0x12, 0x10, 0x11),
        op::log(0x10, 0x11, 0x12, 0x00),
        op::ret(0x20),
    ]
    .into_iter()
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
        maturity,
        0,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program.clone()],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate a checked tx");

    client.deploy(tx);

    let mut script_ops = vec![
        op::movi(0x10, 2048),
        op::aloc(0x10),
        op::move_(0x10, RegId::HP),
        op::movi(0x20, 0x00),
        op::add(0x11, RegId::ZERO, 0x20),
        op::movi(0x12, contract_size as Immediate18),
        op::ccp(0x10, 0x11, RegId::ZERO, 0x12),
        op::addi(0x21, 0x20, ContractId::LEN as Immediate12),
        op::meq(0x30, 0x21, 0x10, 0x12),
        op::ret(0x30),
    ];

    let script = script_ops.clone().into_iter().collect();
    let mut script_data = contract.to_vec();
    script_data.extend(program.as_ref());
    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract);
    let output = Output::contract(0, rng.gen(), rng.gen());

    let mut tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate a checked tx");

    let script_data_mem = client.tx_offset() + tx.transaction().script_data_offset();
    script_ops[3] = op::movi(0x20, script_data_mem as Immediate18);
    let script_mem: Vec<u8> = script_ops.clone().into_iter().collect();

    tx.as_mut()
        .script_mut()
        .as_mut_slice()
        .copy_from_slice(script_mem.as_slice());

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
    let maturity = 0;
    let salt: Salt = rng.gen();
    let height = 0;
    let params = ConsensusParameters::DEFAULT;
    let gas_costs = GasCosts::default();

    let program: Vec<u8> = vec![
        op::movi(0x10, 0x11),
        op::movi(0x11, 0x2a),
        op::add(0x12, 0x10, 0x11),
        op::log(0x10, 0x11, 0x12, 0x00),
        op::ret(0x12),
    ]
    .into_iter()
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
        maturity,
        0,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program],
    )
    .into_checked(height, &params, &gas_costs)
    .expect("failed to generate a checked tx");

    assert!(Transactor::new(&mut storage, Default::default(), gas_costs.clone())
        .transact(tx)
        .is_success());

    let mut script_ops = vec![
        op::movi(0x10, 0x00),
        op::addi(0x11, 0x10, ContractId::LEN as Immediate12),
        op::call(0x10, RegId::ZERO, 0x10, 0x10),
        op::ret(0x30),
    ];

    let script = script_ops.clone().into_iter().collect();
    let mut script_data = contract.to_vec();
    script_data.extend([0u8; WORD_SIZE * 2]);
    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract);
    let output = Output::contract(0, rng.gen(), rng.gen());

    let mut tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, &gas_costs)
    .expect("failed to generate a checked tx");

    let params = ConsensusParameters::default();

    let script_data_mem = params.tx_offset() + tx.transaction().script_data_offset();
    script_ops[0] = op::movi(0x10, script_data_mem as Immediate18);
    let script_mem: Vec<u8> = script_ops.into_iter().collect();

    tx.as_mut()
        .script_mut()
        .as_mut_slice()
        .copy_from_slice(script_mem.as_slice());

    let receipts = Transactor::new(&mut storage, params, gas_costs)
        .transact(tx)
        .receipts()
        .expect("Failed to execute script")
        .to_owned();

    assert_eq!(receipts[0].id(), None);
    assert_eq!(receipts[0].to().expect("Receipt value failed").to_owned(), contract);
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
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;
    let gas_costs = GasCosts::default();

    let salt: Salt = rng.gen();
    let bytecode_witness_index = 0;
    let program: Vec<u8> = vec![
        op::log(RegId::PC, RegId::FP, RegId::SSP, RegId::SP),
        op::noop(),
        op::noop(),
        op::noop(),
        op::noop(),
        op::movi(0x10, 1),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect();

    let contract = Contract::from(program.as_slice());
    let root = contract.root();
    let state_root = Contract::default_state_root();
    let id = contract.id(&salt, &root, &state_root);

    let input = Input::coin_signed(rng.gen(), rng.gen(), 0, rng.gen(), rng.gen(), 0, maturity);
    let output = Output::contract_created(id, state_root);

    let deploy = Transaction::create(
        gas_price,
        gas_limit,
        maturity,
        bytecode_witness_index,
        salt,
        vec![],
        vec![input],
        vec![output],
        vec![program.clone().into()],
    )
    .into_checked_basic(height, &params)
    .expect("failed to generate a checked tx");

    assert!(Transactor::new(&mut storage, Default::default(), gas_costs.clone())
        .transact(deploy)
        .is_success());

    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), id);
    let output = Output::contract(0, rng.gen(), rng.gen());

    let script_len = 16;

    let params = ConsensusParameters::default();

    // Based on the defined script length, we set the appropriate data offset
    let script_data_offset = params.tx_offset() + Script::script_offset_static() + script_len;
    let script_data_offset = script_data_offset as Immediate18;

    let script = vec![
        op::movi(0x10, script_data_offset),
        op::log(RegId::SP, 0, 0, 0),
        op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect::<Vec<u8>>();

    let mut script_data = vec![];

    script_data.extend(id.as_ref());
    script_data.extend(Word::default().to_be_bytes());
    script_data.extend(Word::default().to_be_bytes());

    let script = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, &gas_costs)
    .expect("failed to generate a checked tx");

    let mut vm = Interpreter::with_storage(storage, params, Default::default());

    vm.transact(script).expect("Failed to call deployed contract");

    let asset_id = AssetId::default();
    let contract = Contract::from(program.as_ref());

    let mut frame = CallFrame::new(
        id,
        asset_id,
        [0; VM_REGISTER_COUNT],
        contract.as_ref().len() as Word,
        0,
        0,
    );
    let stack = frame.to_bytes().len() as Word + frame.total_code_size();

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
        test_context.tx_offset()
    );
    let script_data: Vec<u8> = [Call::new(contract_id, 0, 0).to_bytes().as_slice()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // initiate the call to the contract which reverts
    let result = test_context
        .start_script(script_ops, script_data)
        .gas_limit(gas_limit)
        .contract_input(contract_id)
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

#[test]
fn nested_call_limit() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    let salt: Salt = rng.gen();
    let program: Witness = [op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS), op::ret(RegId::ONE)]
        .into_iter()
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
        maturity,
        bytecode_witness,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

    client.deploy(tx);

    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract);
    let output = Output::contract(0, rng.gen(), rng.gen());

    let script: Vec<u8> = vec![
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect();
    let script_data = Call::new(contract, 0, 1000).to_bytes();
    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

    let mut receipts = client.transact(tx).to_vec();

    if let Receipt::ScriptResult { result, .. } = receipts.pop().expect("Missing result receipt") {
        if result != ScriptExecutionResult::Panic {
            panic!("Expected vm panic, got {result:?} instead");
        }
    } else {
        unreachable!("No script result");
    }

    if let Receipt::Panic { reason: pr, .. } = receipts.pop().expect("Missing panic reason receipt") {
        assert_eq!(
            *pr.reason(),
            PanicReason::NestedCallLimitReached,
            "Panic reason differs for the expected reason"
        );
    } else {
        unreachable!("No script receipt for a paniced tx");
    }
}

#[test]
fn revert() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    let salt: Salt = rng.gen();

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
        maturity,
        bytecode_witness,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate a checked tx");

    // Deploy the contract into the blockchain
    client.deploy(tx);

    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract);
    let output = Output::contract(0, rng.gen(), rng.gen());

    // The script needs to locate the data offset at runtime. Hence, we need to know
    // upfront the serialized size of the script so we can set the registers
    // accordingly.
    //
    // This variable is created to assert we have correct script size in the
    // instructions.
    let script_len = 16;

    // Based on the defined script length, we set the appropriate data offset
    let script_data_offset = client.tx_offset() + Script::script_offset_static() + script_len;
    let script_data_offset = script_data_offset as Immediate18;

    let script = vec![
        op::movi(0x10, script_data_offset),
        op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect::<Vec<u8>>();

    // Assert the offsets are set correctnly
    let offset = client.tx_offset() + Script::script_offset_static() + bytes::padded_len(script.as_slice());
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
    script_data.extend(routine.to_be_bytes());
    script_data.extend(call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(val.to_be_bytes());

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate a checked tx");

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
        op::movi(0x10, script_data_offset),
        op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
        op::rvrt(RegId::ONE),
    ]
    .into_iter()
    .collect::<Vec<u8>>();

    let mut script_data = vec![];

    // Value to be added
    let rev: Word = 250;

    // Script data containing the call arguments (contract, a, b) and (key, value)
    script_data.extend(contract.as_ref());
    script_data.extend(routine.to_be_bytes());
    script_data.extend(call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(rev.to_be_bytes());

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate a checked tx");

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

#[test]
fn retd_from_top_of_heap() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    const REG_SIZE: u8 = RegId::WRITABLE.to_u8();
    const REG_PTR: u8 = RegId::WRITABLE.to_u8() + 1;

    #[rustfmt::skip]
    let script = vec![
        op::movi(REG_SIZE, 32),              // Allocate 32 bytes.
        op::aloc(REG_SIZE),                  // $hp -= 32.
        op::move_(REG_PTR, RegId::HP),       // Pointer is $hp, first byte in allocated buffer.
        op::retd(REG_PTR, REG_SIZE),         // Return the allocated buffer.
    ].into_iter()
    .collect::<Vec<u8>>();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, client.gas_costs())
        .expect("failed to generate a checked tx");

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::ReturnData { .. }));
}

#[test]
fn logd_from_top_of_heap() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    const REG_SIZE: RegId = RegId::WRITABLE;
    let reg_ptr: RegId = RegId::new(u8::from(RegId::WRITABLE) + 1);

    #[rustfmt::skip]
    let script = vec![
        op::movi(REG_SIZE, 32),                                               // Allocate 32 bytes.
        op::aloc(REG_SIZE),                                                   // $hp -= 32.
        op::move_(reg_ptr, RegId::HP),                                        // Pointer is $hp, first byte in allocated buffer.
        op::logd(RegId::ZERO, RegId::ZERO, reg_ptr, REG_SIZE), // Log the whole buffer
        op::ret(RegId::ONE),                                                     // Return
    ].into_iter()
    .collect::<Vec<u8>>();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, client.gas_costs())
        .expect("failed to generate a checked tx");

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");

    // Expect the correct receipt
    assert_eq!(receipts.len(), 3);
    assert!(matches!(receipts[0], Receipt::LogData { .. }));
}

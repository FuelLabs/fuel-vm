use fuel_tx::crypto::Hasher;
use fuel_types::bytes;
use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[test]
fn code_copy() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut vm = Interpreter::in_memory();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;

    let salt: Salt = rng.gen();

    let program: Vec<u8> = vec![
        Opcode::ADDI(0x10, REG_ZERO, 0x11),
        Opcode::ADDI(0x11, REG_ZERO, 0x2a),
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
    let contract = contract.id(&salt, &contract_root);

    let contract_size = program.as_ref().len();
    let output = Output::contract_created(contract);

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
    );

    vm.transact(tx).expect("Failed to transact!");

    let mut script_ops = vec![
        Opcode::ADDI(0x10, REG_ZERO, 2048),
        Opcode::ALOC(0x10),
        Opcode::ADDI(0x10, REG_HP, 0x01),
        Opcode::ADDI(0x20, REG_ZERO, 0x00),
        Opcode::ADD(0x11, REG_ZERO, 0x20),
        Opcode::ADDI(0x12, REG_ZERO, contract_size as Immediate12),
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
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    );

    let script_data_mem = VM_TX_MEMORY + tx.script_data_offset().unwrap();
    script_ops[3] = Opcode::ADDI(0x20, REG_ZERO, script_data_mem as Immediate12);
    let script_mem: Vec<u8> = script_ops.iter().copied().collect();

    match &mut tx {
        Transaction::Script { script, .. } => script.as_mut_slice().copy_from_slice(script_mem.as_slice()),
        _ => unreachable!(),
    }

    vm.transact(tx).expect("Failed to transact!");

    assert_eq!(1, vm.registers()[0x30]);
}

#[test]
fn call() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let salt: Salt = rng.gen();

    let program: Vec<u8> = vec![
        Opcode::ADDI(0x10, REG_ZERO, 0x11),
        Opcode::ADDI(0x11, REG_ZERO, 0x2a),
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
    let contract = contract.id(&salt, &contract_root);

    let output = Output::contract_created(contract);

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
    );

    Interpreter::transition(&mut storage, tx).expect("Failed to deploy contract");

    let mut script_ops = vec![
        Opcode::ADDI(0x10, REG_ZERO, 0x00),
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
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    );

    let script_data_mem = VM_TX_MEMORY + tx.script_data_offset().unwrap();
    script_ops[0] = Opcode::ADDI(0x10, REG_ZERO, script_data_mem as Immediate12);
    let script_mem: Vec<u8> = script_ops.iter().copied().collect();

    match &mut tx {
        Transaction::Script { script, .. } => script.as_mut_slice().copy_from_slice(script_mem.as_slice()),
        _ => unreachable!(),
    }

    let state = Interpreter::transition(&mut storage, tx).expect("Failed to execute script");
    let receipt = state.receipts()[1];

    assert_eq!(receipt.ra().expect("Receipt value failed"), 0x11);
    assert_eq!(receipt.rb().expect("Receipt value failed"), 0x2a);
    assert_eq!(receipt.rc().expect("Receipt value failed"), 0x3b);
}

#[test]
fn call_frame_code_offset() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;

    let salt: Salt = rng.gen();
    let bytecode_witness_index = 0;
    let program: Vec<u8> = vec![
        Opcode::LOG(REG_PC, REG_FP, REG_SSP, REG_SP),
        Opcode::NOOP,
        Opcode::NOOP,
        Opcode::NOOP,
        Opcode::NOOP,
        Opcode::ADDI(0x10, REG_ZERO, 1),
        Opcode::RET(REG_ONE),
    ]
    .into_iter()
    .collect();

    let contract = Contract::from(program.as_slice());
    let root = contract.root();
    let id = contract.id(&salt, &root);

    let input = Input::coin(rng.gen(), rng.gen(), 0, rng.gen(), 0, maturity, vec![], vec![]);
    let output = Output::contract_created(id);

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
    );

    Interpreter::transition(&mut storage, deploy).expect("Failed to deploy");

    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), id);
    let output = Output::contract(0, rng.gen(), rng.gen());

    let script_len = 24;

    // Based on the defined script length, we set the appropriate data offset
    let script_data_offset = VM_TX_MEMORY + Transaction::script_offset() + script_len;
    let script_data_offset = script_data_offset as Immediate12;

    let script = vec![
        Opcode::ADDI(0x10, REG_ZERO, script_data_offset),
        Opcode::ADDI(0x11, REG_ZERO, gas_limit as Immediate12),
        Opcode::LOG(REG_SP, 0, 0, 0),
        Opcode::CALL(0x10, REG_ZERO, 0x10, 0x11),
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
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    );

    let mut vm = Interpreter::with_storage(storage);

    vm.transact(script).expect("Failed to call deployed contract");

    let color = Color::default();
    let contract = Contract::from(program.as_ref());

    let mut frame = CallFrame::new(id, color, [0; VM_REGISTER_COUNT], 0, 0, contract);
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
fn revert() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
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
    let contract = contract.id(&salt, &contract_root);

    let output = Output::contract_created(contract);

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
    );

    // Deploy the contract into the blockchain
    client.transition(vec![tx]).expect("Failed to transact");

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
    let script_data_offset = VM_TX_MEMORY + Transaction::script_offset() + script_len;
    let script_data_offset = script_data_offset as Immediate12;

    let script = vec![
        Opcode::ADDI(0x10, REG_ZERO, script_data_offset),
        Opcode::ADDI(0x11, REG_ZERO, gas_limit as Immediate12),
        Opcode::CALL(0x10, REG_ZERO, 0x10, 0x11),
        Opcode::RET(REG_ONE),
    ]
    .iter()
    .copied()
    .collect::<Vec<u8>>();

    // Assert the offsets are set correctnly
    let offset = VM_TX_MEMORY + Transaction::script_offset() + bytes::padded_len(script.as_slice());
    assert_eq!(script_data_offset, offset as Immediate12);

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
        maturity,
        script.clone(),
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    );

    // Assert the initial state of `key` is empty
    let state = client.as_ref().contract_state(&contract, &key);
    assert_eq!(Bytes32::default(), state.into_owned());

    let receipts = client.transition(vec![tx]).expect("Failed to transact");
    let state = client.as_ref().contract_state(&contract, &key);

    // Assert the state of `key` is mutated to `val`
    assert_eq!(&val.to_be_bytes()[..], &state.as_ref()[..WORD_SIZE]);

    // Expect the correct receipt
    assert_eq!(receipts[1].ra().expect("Register value expected"), val);
    assert_eq!(receipts[1].rb().expect("Register value expected"), 0);

    // Create a script with revert instruction
    let script = vec![
        Opcode::ADDI(0x10, REG_ZERO, script_data_offset),
        Opcode::ADDI(0x11, REG_ZERO, gas_limit as Immediate12),
        Opcode::CALL(0x10, REG_ZERO, 0x10, 0x11),
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
        maturity,
        script.clone(),
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    );

    let receipts = client.transition(vec![tx]).expect("Failed to transact");
    let state = client.as_ref().contract_state(&contract, &key);

    // Assert the state of `key` is reverted to `val`
    assert_eq!(&val.to_be_bytes()[..], &state.as_ref()[..WORD_SIZE]);

    // Expect the correct receipt
    assert_eq!(receipts[1].ra().expect("Register value expected"), val + rev);
    assert_eq!(receipts[1].rb().expect("Register value expected"), val);

    match receipts[3] {
        Receipt::Revert { ra, .. } if ra == 1 => (),
        _ => panic!("Expected revert receipt: {:?}", receipts[3]),
    }
}

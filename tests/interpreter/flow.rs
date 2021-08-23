use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[test]
fn code_copy() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let storage = MemoryStorage::default();
    let mut vm = Interpreter::with_storage(storage);

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
        Opcode::ADDI(0x21, 0x20, ContractId::size_of() as Immediate12),
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

    let storage = MemoryStorage::default();
    let mut vm = Interpreter::with_storage(storage);

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

    vm.transact(tx).expect("Failed to transact!");

    let mut script_ops = vec![
        Opcode::ADDI(0x10, REG_ZERO, 0x00),
        Opcode::ADDI(0x11, 0x10, ContractId::size_of() as Immediate12),
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

    vm.transact(tx).expect("Failed to transact!");

    let expected_log = vec![(0x10, 0x11), (0x11, 0x2a), (0x12, 0x3b)];
    expected_log
        .into_iter()
        .enumerate()
        .for_each(|(i, (reg, val))| match vm.log()[i] {
            LogEvent::Register { register, value, .. } => {
                assert_eq!(reg, register);
                assert_eq!(val, value);
            }

            _ => panic!("Unexpected log event!"),
        });
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

    let len = program.len();

    let color = Color::default();
    let contract = Contract::from(program.as_ref());

    let mut frame = CallFrame::new(id, color, [0; VM_REGISTER_COUNT], 0, 0, contract);
    let stack = frame.to_bytes().len();

    let log = vm.log()[0];
    let sp = log.value() as usize;
    assert!(matches!(log, LogEvent::Register { register, .. } if register == REG_SP));

    let log = vm.log()[1];
    let pc = log.value() as usize;
    assert!(matches!(log, LogEvent::Register { register, .. } if register == REG_PC));
    assert_eq!(program.as_slice(), &vm.memory()[pc..pc + len]);

    let log = vm.log()[2];
    let fp = log.value() as usize;
    assert!(matches!(log, LogEvent::Register { register, .. } if register == REG_FP));

    let log = vm.log()[3];
    let ssp = log.value() as usize;
    assert!(matches!(log, LogEvent::Register { register, .. } if register == REG_SSP));

    let log = vm.log()[4];
    let sp_p = log.value() as usize;
    assert!(matches!(log, LogEvent::Register { register, .. } if register == REG_SP));

    assert_eq!(ssp, sp + stack);
    assert_eq!(ssp, fp + stack);
    assert_eq!(ssp, sp_p);
}

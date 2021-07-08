use super::common::{d, r};
use fuel_vm::consts::*;
use fuel_vm::prelude::*;

use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[test]
fn code_copy() {
    let storage = MemoryStorage::default();
    let mut vm = Interpreter::with_storage(storage);

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;

    let salt: Salt = r();

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

    let contract = Contract::from(program.as_ref()).address(salt.as_ref());
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

    vm.init(tx).expect("Failed to init VM with tx create!");
    vm.run().expect("Failed to deploy contract!");

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
    let input = Input::contract(d(), d(), d(), contract);
    let output = Output::contract(0, d(), d());

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

    let script_data_mem = Interpreter::<()>::tx_mem_address() + tx.script_data_offset().unwrap();
    script_ops[3] = Opcode::ADDI(0x20, REG_ZERO, script_data_mem as Immediate12);
    let script_mem: Vec<u8> = script_ops.iter().copied().collect();

    match &mut tx {
        Transaction::Script { script, .. } => script.as_mut_slice().copy_from_slice(script_mem.as_slice()),
        _ => unreachable!(),
    }

    vm.init(tx).expect("Failed to init VM with tx create!");
    vm.run().expect("Failed to execute contract!");

    assert_eq!(1, vm.registers()[0x30]);
}

#[test]
fn call() {
    let storage = MemoryStorage::default();
    let mut vm = Interpreter::with_storage(storage);

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let salt: Salt = r();

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

    let contract = Contract::from(program.as_ref()).address(salt.as_ref());
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

    vm.init(tx).expect("Failed to init VM with tx create!");
    vm.run().expect("Failed to deploy contract!");

    let mut script_ops = vec![
        Opcode::ADDI(0x10, REG_ZERO, 0x00),
        Opcode::ADDI(0x11, 0x10, ContractId::size_of() as Immediate12),
        Opcode::CALL(0x10, REG_ZERO, 0x10, 0x10),
        Opcode::RET(0x30),
    ];

    let script = script_ops.iter().copied().collect();
    let mut script_data = contract.to_vec();
    script_data.extend(&[0u8; WORD_SIZE * 2]);
    let input = Input::contract(d(), d(), d(), contract);
    let output = Output::contract(0, d(), d());

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

    let script_data_mem = Interpreter::<()>::tx_mem_address() + tx.script_data_offset().unwrap();
    script_ops[0] = Opcode::ADDI(0x10, REG_ZERO, script_data_mem as Immediate12);
    let script_mem: Vec<u8> = script_ops.iter().copied().collect();

    match &mut tx {
        Transaction::Script { script, .. } => script.as_mut_slice().copy_from_slice(script_mem.as_slice()),
        _ => unreachable!(),
    }

    vm.init(tx).expect("Failed to init VM with tx create!");
    vm.run().expect("Failed to execute contract!");

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

use super::common::*;
use super::*;

#[test]
fn mint_burn() {
    let mut balance = 1000;

    let storage = MemoryStorage::default();
    let mut vm = Interpreter::with_storage(storage);

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;

    let salt: Salt = r();
    let program: Witness = [
        Opcode::ADDI(0x10, REG_FP, CallFrame::a_offset() as Immediate12),
        Opcode::LW(0x10, 0x10, 0),
        Opcode::ADDI(0x11, REG_FP, CallFrame::b_offset() as Immediate12),
        Opcode::LW(0x11, 0x11, 0),
        Opcode::JNEI(0x10, REG_ZERO, 7),
        Opcode::MINT(0x11),
        Opcode::JI(8),
        Opcode::BURN(0x11),
        Opcode::RET(REG_ONE),
    ]
    .iter()
    .copied()
    .collect::<Vec<u8>>()
    .into();

    let contract = Contract::from(program.as_ref()).address(salt.as_ref());
    let color = Color::from(*contract);
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

    vm.init(tx).expect("Failed to init VM with tx create!");
    vm.run().expect("Failed to deploy contract!");

    let input = Input::contract(d(), d(), d(), contract);
    let output = Output::contract(0, d(), d());

    let mut script_ops = vec![
        Opcode::ADDI(0x10, REG_ZERO, 0),
        Opcode::ADDI(0x11, REG_ZERO, gas_limit as Immediate12),
        Opcode::CALL(0x10, REG_ZERO, 0x10, 0x11),
        Opcode::RET(REG_ONE),
    ];

    let script: Vec<u8> = script_ops.iter().copied().collect();
    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        vec![],
        vec![input.clone()],
        vec![output],
        vec![],
    );

    let script_data_offset = Interpreter::<()>::tx_mem_address() + tx.script_data_offset().unwrap();
    script_ops[0] = Opcode::ADDI(0x10, REG_ZERO, script_data_offset as Immediate12);

    let script: Vec<u8> = script_ops.iter().copied().collect();
    let script_data = Call::new(contract, 0, balance).to_bytes();
    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    );

    assert_eq!(0, vm.balance(&contract, &color).unwrap());
    vm.init(tx).expect("Failed to init VM with tx create!");
    vm.run().expect("Failed to execute contract!");
    assert_eq!(balance as Word, vm.balance(&contract, &color).unwrap());

    // Try to burn more than the available balance
    let script: Vec<u8> = script_ops.iter().copied().collect();
    let script_data = Call::new(contract, 1, balance + 1).to_bytes();
    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    );

    vm.init(tx).expect("Failed to init VM with tx create!");
    assert!(vm.run().is_err());
    assert_eq!(balance as Word, vm.balance(&contract, &color).unwrap());

    // Burn some of the balance
    let burn = 100;

    let script: Vec<u8> = script_ops.iter().copied().collect();
    let script_data = Call::new(contract, 1, burn).to_bytes();
    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    );

    vm.init(tx).expect("Failed to init VM with tx create!");
    vm.run().expect("Failed to execute contract!");
    balance -= burn;
    assert_eq!(balance as Word, vm.balance(&contract, &color).unwrap());

    // Burn the remainder balance
    let script: Vec<u8> = script_ops.iter().copied().collect();
    let script_data = Call::new(contract, 1, balance).to_bytes();
    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    );

    vm.init(tx).expect("Failed to init VM with tx create!");
    vm.run().expect("Failed to execute contract!");
    assert_eq!(0, vm.balance(&contract, &color).unwrap());
}

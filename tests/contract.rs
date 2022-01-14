use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn mint_burn() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut balance = 1000;

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    let salt: Salt = rng.gen();
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

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let contract = contract.id(&salt, &contract_root);

    let color = Color::from(*contract);
    let output = Output::contract_created(contract);

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

    client.transact(tx);

    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), contract);
    let output = Output::contract(0, rng.gen(), rng.gen());

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
        byte_price,
        maturity,
        script,
        vec![],
        vec![input.clone()],
        vec![output],
        vec![],
    );

    let script_data_offset = VM_TX_MEMORY + tx.script_data_offset().unwrap();
    script_ops[0] = Opcode::ADDI(0x10, REG_ZERO, script_data_offset as Immediate12);

    let script: Vec<u8> = script_ops.iter().copied().collect();
    let script_data = Call::new(contract, 0, balance).to_bytes();
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

    let script_data_check_balance: Vec<u8> = color.as_ref().iter().chain(contract.as_ref().iter()).copied().collect();
    let mut script_check_balance = vec![
        Opcode::NOOP,
        Opcode::MOVE(0x11, 0x10),
        Opcode::ADDI(0x12, 0x10, Color::LEN as Immediate12),
        Opcode::BAL(0x10, 0x11, 0x12),
        Opcode::LOG(0x10, REG_ZERO, REG_ZERO, REG_ZERO),
        Opcode::RET(REG_ONE),
    ];

    let tx_check_balance = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script_check_balance.iter().copied().collect(),
        vec![],
        vec![input.clone()],
        vec![output.clone()],
        vec![],
    );

    let script_data_offset = VM_TX_MEMORY + tx_check_balance.script_data_offset().unwrap();
    script_check_balance[0] = Opcode::ADDI(0x10, REG_ZERO, script_data_offset as Immediate12);

    let tx_check_balance = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script_check_balance.into_iter().collect(),
        script_data_check_balance,
        vec![input.clone()],
        vec![output.clone()],
        vec![],
    );

    let storage_balance = client.transact(tx_check_balance.clone())[0]
        .ra()
        .expect("Balance expected");
    assert_eq!(0, storage_balance);

    client.transact(tx);

    let storage_balance = client.transact(tx_check_balance.clone())[0]
        .ra()
        .expect("Balance expected");
    assert_eq!(balance as Word, storage_balance);

    // Try to burn more than the available balance
    let script: Vec<u8> = script_ops.iter().copied().collect();
    let script_data = Call::new(contract, 1, balance + 1).to_bytes();
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

    let storage_balance = client.transact(tx_check_balance.clone())[0]
        .ra()
        .expect("Balance expected");
    assert_eq!(balance as Word, storage_balance);

    // Out of balance test
    client.transact(tx);

    let storage_balance = client.transact(tx_check_balance.clone())[0]
        .ra()
        .expect("Balance expected");
    assert_eq!(balance as Word, storage_balance);

    // Burn some of the balance
    let burn = 100;

    let script: Vec<u8> = script_ops.iter().copied().collect();
    let script_data = Call::new(contract, 1, burn).to_bytes();
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

    client.transact(tx);
    balance -= burn;

    let storage_balance = client.transact(tx_check_balance.clone())[0]
        .ra()
        .expect("Balance expected");
    assert_eq!(balance as Word, storage_balance);

    // Burn the remainder balance
    let script: Vec<u8> = script_ops.iter().copied().collect();
    let script_data = Call::new(contract, 1, balance).to_bytes();
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

    client.transact(tx);

    let storage_balance = client.transact(tx_check_balance.clone())[0]
        .ra()
        .expect("Balance expected");
    assert_eq!(0, storage_balance);
}

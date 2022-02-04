use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use fuel_vm::script_with_data_offset;
use fuel_vm::util::test_helpers::TestBuilder;
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
    let state_root = Contract::initial_state_root(&[]);
    let contract = contract.id(&salt, &contract_root);

    let color = Color::from(*contract);
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

#[test]
fn internal_transfer_reduces_source_contract_balance_and_increases_destination_contract_balance() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_limit = 1_000_000;
    let asset_id: Color = rng.gen();
    let transfer_amount = 500;
    let initial_internal_balance = 1_000_000;

    let mut test_context = TestBuilder::new(2322u64);
    let dest_contract_id = test_context.setup_contract(vec![], None, None).contract_id;

    let program = vec![
        // load amount of tokens
        Opcode::ADDI(0x10, REG_FP, CallFrame::a_offset() as Immediate12),
        Opcode::LW(0x10, 0x10, 0),
        // load color
        Opcode::ADDI(0x11, REG_FP, CallFrame::b_offset() as Immediate12),
        Opcode::LW(0x11, 0x11, 0),
        // load contract id
        Opcode::ADDI(0x12, 0x11, 32 as Immediate12),
        Opcode::TR(0x12, 0x10, 0x11),
        Opcode::RET(REG_ONE),
    ];
    let sender_contract_id = test_context
        .setup_contract(program, Some((asset_id, initial_internal_balance)), None)
        .contract_id;

    let (script_ops, offset) = script_with_data_offset!(
        data_offset,
        vec![
            // load call data to 0x10
            Opcode::ADDI(0x10, REG_ZERO, data_offset + 64),
            // load gas forward to 0x11
            Opcode::ADDI(0x11, REG_ZERO, gas_limit as Immediate12),
            // call the transfer contract
            Opcode::CALL(0x10, REG_ZERO, REG_ZERO, 0x11),
            Opcode::RET(REG_ONE),
        ]
    );
    let script_data: Vec<u8> = [
        asset_id.as_ref(),
        dest_contract_id.as_ref(),
        Call::new(sender_contract_id, transfer_amount, offset as Word)
            .to_bytes()
            .as_slice(),
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();

    // assert initial balance state
    let dest_balance = test_context.get_contract_balance(&dest_contract_id, &asset_id);
    assert_eq!(dest_balance, 0);
    let source_balance = test_context.get_contract_balance(&sender_contract_id, &asset_id);
    assert_eq!(source_balance, initial_internal_balance);

    // initiate the transfer between contracts
    let transfer_tx = test_context
        .gas_limit(gas_limit)
        .gas_price(0)
        .byte_price(0)
        .contract_input(sender_contract_id)
        .contract_input(dest_contract_id)
        .contract_output(&sender_contract_id)
        .contract_output(&dest_contract_id)
        .script(script_ops)
        .script_data(script_data)
        .execute();

    // Ensure transfer tx processed correctly
    assert!(!transfer_tx.should_revert());

    // verify balance transfer occurred
    let dest_balance = test_context.get_contract_balance(&dest_contract_id, &asset_id);
    assert_eq!(dest_balance, transfer_amount);
    let source_balance = test_context.get_contract_balance(&sender_contract_id, &asset_id);
    assert_eq!(source_balance, initial_internal_balance - transfer_amount);
}

#[test]
fn internal_transfer_cant_exceed_more_than_source_contract_balance() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_limit = 1_000_000;
    let asset_id: Color = rng.gen();
    let transfer_amount = 500;
    // set initial internal balance to < transfer amount
    let initial_internal_balance = 100;

    let mut test_context = TestBuilder::new(2322u64);
    let dest_contract_id = test_context.setup_contract(vec![], None, None).contract_id;

    let program = vec![
        // load amount of tokens
        Opcode::ADDI(0x10, REG_FP, CallFrame::a_offset() as Immediate12),
        Opcode::LW(0x10, 0x10, 0),
        // load color
        Opcode::ADDI(0x11, REG_FP, CallFrame::b_offset() as Immediate12),
        Opcode::LW(0x11, 0x11, 0),
        // load contract id
        Opcode::ADDI(0x12, 0x11, 32 as Immediate12),
        Opcode::TR(0x12, 0x10, 0x11),
        Opcode::RET(REG_ONE),
    ];

    let sender_contract_id = test_context
        .setup_contract(program, Some((asset_id, initial_internal_balance)), None)
        .contract_id;

    let (script_ops, offset) = script_with_data_offset!(
        data_offset,
        vec![
            // load call data to 0x10
            Opcode::ADDI(0x10, REG_ZERO, data_offset + 64),
            // load gas forward to 0x11
            Opcode::ADDI(0x11, REG_ZERO, gas_limit as Immediate12),
            // call the transfer contract
            Opcode::CALL(0x10, REG_ZERO, REG_ZERO, 0x11),
            Opcode::RET(REG_ONE),
        ]
    );
    let script_data: Vec<u8> = [
        asset_id.as_ref(),
        dest_contract_id.as_ref(),
        Call::new(sender_contract_id, transfer_amount, offset as Word)
            .to_bytes()
            .as_slice(),
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();

    // assert initial balance state
    let dest_balance = test_context.get_contract_balance(&dest_contract_id, &asset_id);
    assert_eq!(dest_balance, 0);
    let source_balance = test_context.get_contract_balance(&sender_contract_id, &asset_id);
    assert_eq!(source_balance, initial_internal_balance);

    let transfer_tx = test_context
        .gas_limit(gas_limit)
        .gas_price(0)
        .byte_price(0)
        .contract_input(sender_contract_id)
        .contract_input(dest_contract_id)
        .contract_output(&sender_contract_id)
        .contract_output(&dest_contract_id)
        .script(script_ops)
        .script_data(script_data)
        .execute();

    // Ensure transfer tx reverts since transfer amount is too large
    assert!(transfer_tx.should_revert());

    // verify balance transfer did not occur
    let dest_balance = test_context.get_contract_balance(&dest_contract_id, &asset_id);
    assert_eq!(dest_balance, 0);
    let source_balance = test_context.get_contract_balance(&sender_contract_id, &asset_id);
    assert_eq!(source_balance, initial_internal_balance);
}

use fuel_asm::op;
use fuel_tx::field::ScriptData;
use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use fuel_vm::script_with_data_offset;
use fuel_vm::util::test_helpers::TestBuilder;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn prevent_contract_id_redeployment() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let mut client = MemoryClient::default();

    let input_amount = 1000;
    let gas_price = 0;
    let spend_amount = 600;
    let asset_id = AssetId::BASE;

    #[rustfmt::skip]
        let function_undefined: Vec<Opcode> = vec![
        Opcode::Undefined,
    ];

    let salt: Salt = rng.gen();
    let program: Witness = function_undefined.into_iter().collect::<Vec<u8>>().into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract_undefined = contract.id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract_undefined, state_root);

    let mut create = Transaction::create(
        gas_price,
        0,
        0,
        0,
        salt,
        vec![],
        vec![],
        vec![
            output,
            Output::change(rng.gen(), 0, asset_id),
            Output::coin(rng.gen(), spend_amount, asset_id),
        ],
        vec![program],
    );
    create.add_unsigned_coin_input(rng.gen(), &Default::default(), input_amount, asset_id, rng.gen(), 0);
    let create = create
        .into_checked_basic(1, &ConsensusParameters::default())
        .expect("failed to generate checked tx");

    // deploy contract
    client.deploy(create.clone()).expect("First create should be executed");
    let mut txtor: Transactor<_, _> = client.into();
    // second deployment should fail
    let result = txtor.deploy(create).unwrap_err();
    assert_eq!(result, InterpreterError::Panic(PanicReason::ContractIdAlreadyDeployed));
}

#[test]
fn mint_burn() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut balance = 1000;

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    let salt: Salt = rng.gen();
    let program: Witness = [
        op::addi(0x10, REG_FP.into(), CallFrame::a_offset() as Immediate12),
        op::lw(0x10, 0x10, 0),
        op::addi(0x11, REG_FP.into(), CallFrame::b_offset() as Immediate12),
        op::lw(0x11, 0x11, 0),
        op::jnei(0x10, REG_ZERO.into(), 7),
        op::mint(0x11),
        op::ji(8),
        op::burn(0x11),
        op::ret(REG_ONE.into()),
    ]
    .into_iter()
    .collect::<Vec<u8>>()
    .into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract = contract.id(&salt, &contract_root, &state_root);

    let asset_id = AssetId::from(*contract);
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

    let mut script_ops = vec![
        op::movi(0x10, 0),
        op::call(0x10, REG_ZERO.into(), 0x10, REG_CGAS.into()),
        op::ret(REG_ONE.into()),
    ];

    let script: Vec<u8> = script_ops.clone().into_iter().collect();
    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        vec![],
        vec![input.clone()],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

    let script_data_offset = client.tx_offset() + tx.transaction().script_data_offset();
    script_ops[0] = op::movi(0x10, script_data_offset as Immediate18);

    let script: Vec<u8> = script_ops.clone().into_iter().collect();
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
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

    let script_data_check_balance: Vec<u8> = asset_id
        .as_ref()
        .iter()
        .chain(contract.as_ref().iter())
        .copied()
        .collect();
    let mut script_check_balance = vec![
        op::noop(),
        op::move_(0x11, 0x10),
        op::addi(0x12, 0x10, AssetId::LEN as Immediate12),
        op::bal(0x10, 0x11, 0x12),
        op::log(0x10, REG_ZERO.into(), REG_ZERO.into(), REG_ZERO.into()),
        op::ret(REG_ONE.into()),
    ];

    let tx_check_balance = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script_check_balance.clone().into_iter().collect(),
        vec![],
        vec![input.clone()],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

    let script_data_offset = client.tx_offset() + tx_check_balance.transaction().script_data_offset();
    script_check_balance[0] = op::movi(0x10, script_data_offset as Immediate18);

    let tx_check_balance = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script_check_balance.into_iter().collect(),
        script_data_check_balance,
        vec![input.clone()],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

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
    let script: Vec<u8> = script_ops.clone().into_iter().collect();
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
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

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

    let script: Vec<u8> = script_ops.clone().into_iter().collect();
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
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

    client.transact(tx);
    balance -= burn;

    let storage_balance = client.transact(tx_check_balance.clone())[0]
        .ra()
        .expect("Balance expected");
    assert_eq!(balance as Word, storage_balance);

    // Burn the remainder balance
    let script: Vec<u8> = script_ops.into_iter().collect();
    let script_data = Call::new(contract, 1, balance).to_bytes();
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

    client.transact(tx);

    let storage_balance = client.transact(tx_check_balance)[0].ra().expect("Balance expected");
    assert_eq!(0, storage_balance);
}

#[test]
fn call_increases_contract_asset_balance_and_balance_register() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();
    let call_amount = 500u64;

    let mut test_context = TestBuilder::new(2322u64);
    let contract_id = test_context
        .setup_contract(vec![op::ret(REG_BAL.into())], None, None)
        .contract_id;

    let (script_ops, offset) = script_with_data_offset!(
        data_offset,
        vec![
            // load call data to 0x10
            op::movi(0x10, data_offset + 32),
            // load balance to forward to 0x12
            op::movi(0x11, call_amount as Immediate18),
            // load the asset id to use to 0x13
            op::movi(0x12, data_offset),
            // call the transfer contract
            op::call(0x10, 0x11, 0x12, REG_CGAS.into()),
            op::ret(REG_ONE.into()),
        ],
        test_context.tx_offset()
    );
    let script_data: Vec<u8> = [
        asset_id.as_ref(),
        Call::new(contract_id, 0, offset as Word).to_bytes().as_slice(),
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();

    // starting contract balance
    let start_balance = test_context.get_contract_balance(&contract_id, &asset_id);
    assert_eq!(start_balance, 0);

    // call contract with some amount of coins to forward
    let transfer_tx = test_context
        .start_script(script_ops, script_data)
        .gas_limit(gas_limit)
        .gas_price(0)
        .coin_input(asset_id, call_amount)
        .contract_input(contract_id)
        .contract_output(&contract_id)
        .change_output(asset_id)
        .execute();

    // Ensure transfer tx processed correctly
    assert!(!transfer_tx.should_revert());

    // verify balance transfer occurred
    let end_balance = test_context.get_contract_balance(&contract_id, &asset_id);
    assert_eq!(end_balance, call_amount);

    // verify balance register was set
    assert_eq!(transfer_tx.receipts()[1].val().unwrap(), call_amount);
}

#[test]
fn call_decreases_internal_balance_and_increases_destination_contract_balance() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();
    let call_amount = 500;
    let initial_internal_balance = 1_000_000;

    let mut test_context = TestBuilder::new(2322u64);
    let dest_contract_id = test_context
        .setup_contract(
            vec![
                // log the balance register
                op::ret(REG_BAL.into()),
            ],
            None,
            None,
        )
        .contract_id;

    let program = vec![
        // load amount of tokens
        op::addi(0x10, REG_FP.into(), CallFrame::a_offset() as Immediate12),
        op::lw(0x10, 0x10, 0),
        // load asset id
        op::addi(0x11, REG_FP.into(), CallFrame::b_offset() as Immediate12),
        op::lw(0x11, 0x11, 0),
        // load contract id
        op::addi(0x12, 0x11, 32 as Immediate12),
        op::call(0x12, 0x10, 0x11, REG_CGAS.into()),
        op::ret(REG_BAL.into()),
    ];
    let sender_contract_id = test_context
        .setup_contract(program, Some((asset_id, initial_internal_balance)), None)
        .contract_id;

    let (script_ops, offset) = script_with_data_offset!(
        data_offset,
        vec![
            // load call data to 0x10
            op::movi(0x10, data_offset + 64),
            // call the transfer contract
            op::call(0x10, REG_ZERO.into(), REG_ZERO.into(), REG_CGAS.into()),
            op::ret(REG_ONE.into()),
        ],
        test_context.tx_offset()
    );
    let script_data: Vec<u8> = [
        asset_id.as_ref(),
        dest_contract_id.as_ref(),
        Call::new(sender_contract_id, call_amount, offset as Word)
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

    // initiate the call between contracts
    let transfer_tx = test_context
        .start_script(script_ops, script_data)
        .gas_limit(gas_limit)
        .gas_price(0)
        .contract_input(sender_contract_id)
        .contract_input(dest_contract_id)
        .contract_output(&sender_contract_id)
        .contract_output(&dest_contract_id)
        .execute();

    // Ensure transfer tx processed correctly
    assert!(!transfer_tx.should_revert());

    // verify balance transfer occurred
    let dest_balance = test_context.get_contract_balance(&dest_contract_id, &asset_id);
    assert_eq!(dest_balance, call_amount);
    let source_balance = test_context.get_contract_balance(&sender_contract_id, &asset_id);
    assert_eq!(source_balance, initial_internal_balance - call_amount);

    // verify balance register of source contract
    // should be zero because external call transferred nothing
    assert_eq!(transfer_tx.receipts()[3].val().unwrap(), 0);

    // verify balance register of destination contract
    assert_eq!(transfer_tx.receipts()[2].val().unwrap(), call_amount);
}

#[test]
fn internal_transfer_reduces_source_contract_balance_and_increases_destination_contract_balance() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();
    let transfer_amount = 500;
    let initial_internal_balance = 1_000_000;

    let mut test_context = TestBuilder::new(2322u64);
    let dest_contract_id = test_context.setup_contract(vec![], None, None).contract_id;

    let program = vec![
        // load amount of tokens
        op::addi(0x10, REG_FP.into(), CallFrame::a_offset() as Immediate12),
        op::lw(0x10, 0x10, 0),
        // load asset id
        op::addi(0x11, REG_FP.into(), CallFrame::b_offset() as Immediate12),
        op::lw(0x11, 0x11, 0),
        // load contract id
        op::addi(0x12, 0x11, 32 as Immediate12),
        op::tr(0x12, 0x10, 0x11),
        op::ret(REG_ONE.into()),
    ];
    let sender_contract_id = test_context
        .setup_contract(program, Some((asset_id, initial_internal_balance)), None)
        .contract_id;

    let (script_ops, offset) = script_with_data_offset!(
        data_offset,
        vec![
            // load call data to 0x10
            op::movi(0x10, data_offset + 64),
            // call the transfer contract
            op::call(0x10, REG_ZERO.into(), REG_ZERO.into(), REG_CGAS.into()),
            op::ret(REG_ONE.into()),
        ],
        test_context.tx_offset()
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
        .start_script(script_ops, script_data)
        .gas_limit(gas_limit)
        .gas_price(0)
        .contract_input(sender_contract_id)
        .contract_input(dest_contract_id)
        .contract_output(&sender_contract_id)
        .contract_output(&dest_contract_id)
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
    let asset_id: AssetId = rng.gen();
    let transfer_amount = 500;
    // set initial internal balance to < transfer amount
    let initial_internal_balance = 100;

    let mut test_context = TestBuilder::new(2322u64);
    let dest_contract_id = test_context.setup_contract(vec![], None, None).contract_id;

    let program = vec![
        // load amount of tokens
        op::addi(0x10, REG_FP.into(), CallFrame::a_offset() as Immediate12),
        op::lw(0x10, 0x10, 0),
        // load asset id
        op::addi(0x11, REG_FP.into(), CallFrame::b_offset() as Immediate12),
        op::lw(0x11, 0x11, 0),
        // load contract id
        op::addi(0x12, 0x11, 32 as Immediate12),
        op::tr(0x12, 0x10, 0x11),
        op::ret(REG_ONE.into()),
    ];

    let sender_contract_id = test_context
        .setup_contract(program, Some((asset_id, initial_internal_balance)), None)
        .contract_id;

    let (script_ops, offset) = script_with_data_offset!(
        data_offset,
        vec![
            // load call data to 0x10
            op::movi(0x10, data_offset + 64),
            // call the transfer contract
            op::call(0x10, REG_ZERO.into(), REG_ZERO.into(), REG_CGAS.into()),
            op::ret(REG_ONE.into()),
        ],
        test_context.tx_offset()
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
        .start_script(script_ops, script_data)
        .gas_limit(gas_limit)
        .gas_price(0)
        .contract_input(sender_contract_id)
        .contract_input(dest_contract_id)
        .contract_output(&sender_contract_id)
        .contract_output(&dest_contract_id)
        .execute();

    // Ensure transfer tx reverts since transfer amount is too large
    assert!(transfer_tx.should_revert());

    // verify balance transfer did not occur
    let dest_balance = test_context.get_contract_balance(&dest_contract_id, &asset_id);
    assert_eq!(dest_balance, 0);
    let source_balance = test_context.get_contract_balance(&sender_contract_id, &asset_id);
    assert_eq!(source_balance, initial_internal_balance);
}

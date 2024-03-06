use crate::{
    prelude::*,
    script_with_data_offset,
    util::test_helpers::TestBuilder,
};
use alloc::{
    vec,
    vec::Vec,
};
use fuel_asm::{
    op,
    RegId,
};
use fuel_tx::{
    policies::Policies,
    ConsensusParameters,
    Witness,
};
use fuel_types::canonical::Serialize;
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};

#[test]
fn prevent_contract_id_redeployment() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let mut client = MemoryClient::default();

    let input_amount = 1000;
    let spend_amount = 600;
    let asset_id = AssetId::BASE;

    #[rustfmt::skip]
    let function_rvrt: Vec<Instruction> = vec![
        op::rvrt(0),
    ];

    let salt: Salt = rng.gen();
    let program: Witness = function_rvrt.into_iter().collect::<Vec<u8>>().into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract_undefined = contract.id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract_undefined, state_root);

    let policies = Policies::new().with_max_fee(0);

    let mut create = Transaction::create(
        Default::default(),
        policies,
        salt,
        vec![],
        vec![],
        vec![
            output,
            Output::change(rng.gen(), 0, asset_id),
            Output::coin(rng.gen(), spend_amount, asset_id),
        ],
        vec![program, Witness::default()],
    );
    create.add_unsigned_coin_input(
        rng.gen(),
        &Default::default(),
        input_amount,
        asset_id,
        rng.gen(),
        Default::default(),
    );

    let consensus_params = ConsensusParameters::standard();

    let create = create
        .into_checked_basic(1.into(), &consensus_params)
        .expect("failed to generate checked tx");

    // deploy contract
    client
        .deploy(create.clone())
        .expect("First create should be executed");
    let mut txtor: Transactor<_, _> = client.into();
    // second deployment should fail
    let result = txtor.deploy(create).unwrap_err();
    assert_eq!(
        result,
        InterpreterError::Panic(PanicReason::ContractIdAlreadyDeployed)
    );
}

#[test]
fn mint_burn() {
    let mut test_context = TestBuilder::new(2322u64);

    let mut balance = 1000;
    let gas_limit = 1_000_000;

    let program = vec![
        op::addi(0x10, RegId::FP, CallFrame::a_offset() as Immediate12),
        op::lw(0x10, 0x10, 0),
        op::addi(0x11, RegId::FP, CallFrame::b_offset() as Immediate12),
        op::lw(0x11, 0x11, 0),
        // Allocate 32 bytes for the zeroed `sub_id`.
        op::movi(0x15, Bytes32::LEN as u32),
        op::aloc(0x15),
        op::jnei(0x10, RegId::ZERO, 9),
        // Mint `0x11` amount of an assets created from zeroed `sub_id`
        op::mint(0x11, RegId::HP),
        op::ji(10),
        op::burn(0x11, RegId::HP),
        op::ret(RegId::ONE),
    ];

    let contract_id = test_context.setup_contract(program, None, None).contract_id;

    let asset_id = contract_id.asset_id(&Bytes32::zeroed());

    let (script_call, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );
    let script_call_data = Call::new(contract_id, 0, balance).to_bytes();

    let script_data_check_balance: Vec<u8> = asset_id
        .as_ref()
        .iter()
        .chain(contract_id.as_ref().iter())
        .copied()
        .collect();

    let (script_check_balance, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::move_(0x11, 0x10),
            op::addi(0x12, 0x10, AssetId::LEN as Immediate12),
            op::bal(0x10, 0x11, 0x12),
            op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let result = test_context
        .start_script(
            script_check_balance.clone(),
            script_data_check_balance.clone(),
        )
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let storage_balance = result.receipts()[0].ra().expect("Balance expected");
    assert_eq!(0, storage_balance);

    test_context
        .start_script(script_call.clone(), script_call_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let result = test_context
        .start_script(
            script_check_balance.clone(),
            script_data_check_balance.clone(),
        )
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let storage_balance = result.receipts()[0].ra().expect("Balance expected");
    assert_eq!(balance as Word, storage_balance);

    // Try to burn more than the available balance
    let script_call_data = Call::new(contract_id, 1, balance + 1).to_bytes();

    let result = test_context
        .start_script(script_call.clone(), script_call_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();
    assert!(result.should_revert());

    let result = test_context
        .start_script(
            script_check_balance.clone(),
            script_data_check_balance.clone(),
        )
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let storage_balance = result.receipts()[0].ra().expect("Balance expected");
    assert_eq!(balance as Word, storage_balance);

    // Burn some of the balance
    let burn = 100;

    let script_call_data = Call::new(contract_id, 1, burn).to_bytes();
    test_context
        .start_script(script_call.clone(), script_call_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();
    balance -= burn;

    let result = test_context
        .start_script(
            script_check_balance.clone(),
            script_data_check_balance.clone(),
        )
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let storage_balance = result.receipts()[0].ra().expect("Balance expected");
    assert_eq!(balance as Word, storage_balance);

    // Burn the remainder balance
    let script_call_data = Call::new(contract_id, 1, balance).to_bytes();
    test_context
        .start_script(script_call, script_call_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let result = test_context
        .start_script(script_check_balance, script_data_check_balance)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let storage_balance = result.receipts()[0].ra().expect("Balance expected");
    assert_eq!(0, storage_balance);
}

#[test]
fn mint_consumes_gas_for_new_assets() {
    let mut test_context = TestBuilder::new(2322u64);

    let balance = 1000;
    let gas_limit = 1_000_000;

    let [new_asset, existing_asset] = [true, false].map(|create_new_asset| {
        let mut program = vec![
            op::addi(0x10, RegId::FP, CallFrame::a_offset() as Immediate12),
            op::lw(0x10, 0x10, 0),
            op::addi(0x11, RegId::FP, CallFrame::b_offset() as Immediate12),
            op::lw(0x11, 0x11, 0),
            // Allocate 32 bytes for the zeroed `sub_id`.
            op::movi(0x15, Bytes32::LEN as u32),
            op::aloc(0x15),
        ];

        // Mint some of the asset before to make the asset exist before the measured mint
        if !create_new_asset {
            program.push(op::mint(0x11, RegId::HP));
        }

        // The mint we're measuring
        program.extend([
            op::log(RegId::GGAS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
            op::mint(0x11, RegId::HP),
            op::log(RegId::GGAS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
            op::ret(RegId::ONE),
        ]);

        let contract_id = test_context.setup_contract(program, None, None).contract_id;

        let (script_call, _) = script_with_data_offset!(
            data_offset,
            vec![
                op::movi(0x10, data_offset as Immediate18),
                op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
                op::ret(RegId::ONE),
            ],
            test_context.get_tx_params().tx_offset()
        );
        let script_call_data = Call::new(contract_id, 0, balance).to_bytes();

        let result = test_context
            .start_script(script_call.clone(), script_call_data)
            .script_gas_limit(gas_limit)
            .contract_input(contract_id)
            .fee_input()
            .contract_output(&contract_id)
            .execute();

        let mut gas_values = result.receipts().iter().filter_map(|v| match v {
            Receipt::Log { ra, .. } => Some(ra),
            _ => None,
        });

        let gas_before = gas_values.next().expect("Missing log receipt");
        let gas_after = gas_values.next().expect("Missing log receipt");
        gas_before - gas_after
    });

    assert!(new_asset > existing_asset);
}

#[test]
fn call_increases_contract_asset_balance_and_balance_register() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();
    let call_amount = 500u64;

    let mut test_context = TestBuilder::new(2322u64);
    let contract_id = test_context
        .setup_contract(vec![op::ret(RegId::BAL)], None, None)
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
            op::call(0x10, 0x11, 0x12, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );
    let script_data: Vec<u8> = [
        asset_id.as_ref(),
        Call::new(contract_id, 0, offset as Word)
            .to_bytes()
            .as_slice(),
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
        .script_gas_limit(gas_limit)
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
                op::ret(RegId::BAL),
            ],
            None,
            None,
        )
        .contract_id;

    let program = vec![
        // load amount of tokens
        op::addi(0x10, RegId::FP, CallFrame::a_offset() as Immediate12),
        op::lw(0x10, 0x10, 0),
        // load asset id
        op::addi(0x11, RegId::FP, CallFrame::b_offset() as Immediate12),
        op::lw(0x11, 0x11, 0),
        // load contract id
        op::addi(0x12, 0x11, 32 as Immediate12),
        op::call(0x12, 0x10, 0x11, RegId::CGAS),
        op::ret(RegId::BAL),
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
            op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
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
    let source_balance =
        test_context.get_contract_balance(&sender_contract_id, &asset_id);
    assert_eq!(source_balance, initial_internal_balance);

    // initiate the call between contracts
    let transfer_tx = test_context
        .start_script(script_ops, script_data)
        .script_gas_limit(gas_limit)
        .gas_price(0)
        .contract_input(sender_contract_id)
        .contract_input(dest_contract_id)
        .fee_input()
        .contract_output(&sender_contract_id)
        .contract_output(&dest_contract_id)
        .execute();

    // Ensure transfer tx processed correctly
    assert!(!transfer_tx.should_revert());

    // verify balance transfer occurred
    let dest_balance = test_context.get_contract_balance(&dest_contract_id, &asset_id);
    assert_eq!(dest_balance, call_amount);
    let source_balance =
        test_context.get_contract_balance(&sender_contract_id, &asset_id);
    assert_eq!(source_balance, initial_internal_balance - call_amount);

    // verify balance register of source contract
    // should be zero because external call transferred nothing
    assert_eq!(transfer_tx.receipts()[3].val().unwrap(), 0);

    // verify balance register of destination contract
    assert_eq!(transfer_tx.receipts()[2].val().unwrap(), call_amount);
}

#[test]
fn internal_transfer_reduces_source_contract_balance_and_increases_destination_contract_balance(
) {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();
    let transfer_amount = 500;
    let initial_internal_balance = 1_000_000;

    let mut test_context = TestBuilder::new(2322u64);
    let dest_contract_id = test_context.setup_contract(vec![], None, None).contract_id;

    let program = vec![
        // load amount of tokens
        op::addi(0x10, RegId::FP, CallFrame::a_offset() as Immediate12),
        op::lw(0x10, 0x10, 0),
        // load asset id
        op::addi(0x11, RegId::FP, CallFrame::b_offset() as Immediate12),
        op::lw(0x11, 0x11, 0),
        // load contract id
        op::addi(0x12, 0x11, 32 as Immediate12),
        op::tr(0x12, 0x10, 0x11),
        op::ret(RegId::ONE),
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
            op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
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
    let source_balance =
        test_context.get_contract_balance(&sender_contract_id, &asset_id);
    assert_eq!(source_balance, initial_internal_balance);

    // initiate the transfer between contracts
    let transfer_tx = test_context
        .start_script(script_ops, script_data)
        .script_gas_limit(gas_limit)
        .gas_price(0)
        .contract_input(sender_contract_id)
        .contract_input(dest_contract_id)
        .fee_input()
        .contract_output(&sender_contract_id)
        .contract_output(&dest_contract_id)
        .execute();

    // Ensure transfer tx processed correctly
    assert!(!transfer_tx.should_revert());

    // verify balance transfer occurred
    let dest_balance = test_context.get_contract_balance(&dest_contract_id, &asset_id);
    assert_eq!(dest_balance, transfer_amount);
    let source_balance =
        test_context.get_contract_balance(&sender_contract_id, &asset_id);
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
        op::addi(0x10, RegId::FP, CallFrame::a_offset() as Immediate12),
        op::lw(0x10, 0x10, 0),
        // load asset id
        op::addi(0x11, RegId::FP, CallFrame::b_offset() as Immediate12),
        op::lw(0x11, 0x11, 0),
        // load contract id
        op::addi(0x12, 0x11, 32 as Immediate12),
        op::tr(0x12, 0x10, 0x11),
        op::ret(RegId::ONE),
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
            op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
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
    let source_balance =
        test_context.get_contract_balance(&sender_contract_id, &asset_id);
    assert_eq!(source_balance, initial_internal_balance);

    let transfer_tx = test_context
        .start_script(script_ops, script_data)
        .script_gas_limit(gas_limit)
        .gas_price(0)
        .contract_input(sender_contract_id)
        .contract_input(dest_contract_id)
        .fee_input()
        .contract_output(&sender_contract_id)
        .contract_output(&dest_contract_id)
        .execute();

    // Ensure transfer tx reverts since transfer amount is too large
    assert!(transfer_tx.should_revert());

    // verify balance transfer did not occur
    let dest_balance = test_context.get_contract_balance(&dest_contract_id, &asset_id);
    assert_eq!(dest_balance, 0);
    let source_balance =
        test_context.get_contract_balance(&sender_contract_id, &asset_id);
    assert_eq!(source_balance, initial_internal_balance);
}

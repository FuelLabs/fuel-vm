use alloc::{
    borrow::ToOwned,
    vec,
    vec::Vec,
};

use crate::{
    prelude::{
        field::Outputs,
        *,
    },
    script_with_data_offset,
    util::test_helpers::{
        find_change,
        TestBuilder,
    },
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

/// Testing of post-execution output handling
#[test]
fn full_change_with_no_fees() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let input_amount = 1000;
    let gas_price = 0;
    let base_asset_id: AssetId = rng.gen();

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .coin_input(base_asset_id, input_amount)
        .change_output(base_asset_id)
        .execute_get_change(base_asset_id);

    assert_eq!(change, input_amount);
}

#[test]
fn used_gas_is_deducted_from_base_asset_change() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let base_asset_id = rng.gen();
    let input_amount = 1000;
    let gas_price = 1;

    let change = TestBuilder::new(2322u64)
        .max_fee_limit(1000)
        .gas_price(gas_price)
        .base_asset_id(base_asset_id)
        .coin_input(base_asset_id, input_amount)
        .change_output(base_asset_id)
        .execute_get_change(base_asset_id);

    assert!(change < input_amount);
}

#[test]
fn used_gas_is_deducted_from_base_asset_change_on_revert() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let base_asset_id = rng.gen();
    let input_amount = 1000;
    let gas_price = 1;

    let change = TestBuilder::new(2322u64)
        .start_script(
            vec![
                // Log some dummy data to burn extra gas
                op::log(RegId::ONE, RegId::ONE, RegId::ONE, RegId::ONE),
                // Revert transaction
                op::rvrt(RegId::ONE),
            ]
            .into_iter()
            .collect(),
            vec![],
        )
        .max_fee_limit(1000)
        .gas_price(gas_price)
        .base_asset_id(base_asset_id)
        .coin_input(base_asset_id, input_amount)
        .change_output(base_asset_id)
        .execute_get_change(base_asset_id);

    assert!(change < input_amount);
}

#[test]
fn correct_change_is_provided_for_coin_outputs_script() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let input_amount = 1000;
    let gas_price = 0;
    let spend_amount = 600;
    let asset_id: AssetId = rng.gen();

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .coin_input(asset_id, input_amount)
        .change_output(asset_id)
        .coin_output(asset_id, spend_amount)
        .execute_get_change(asset_id);

    assert_eq!(change, input_amount - spend_amount);
}

#[test]
fn correct_change_is_provided_for_coin_outputs_create() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let input_amount = 1000;
    let spend_amount = 600;
    let base_asset_id: AssetId = rng.gen();

    #[rustfmt::skip]
    let invalid_instruction_bytecode = vec![0u8; 4];

    let salt: Salt = rng.gen();
    let program: Witness = invalid_instruction_bytecode.into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract_undefined = contract.id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract_undefined, state_root);

    let mut context = TestBuilder::new(2322u64);
    let context = context.base_asset_id(base_asset_id);
    let bytecode_witness = 0;

    let mut create = Transaction::create(
        bytecode_witness,
        Policies::new().with_max_fee(0),
        salt,
        vec![],
        vec![],
        vec![
            output,
            Output::change(rng.gen(), 0, base_asset_id),
            Output::coin(rng.gen(), spend_amount, base_asset_id),
        ],
        vec![program, Witness::default()],
    );
    create.add_unsigned_coin_input(
        rng.gen(),
        &Default::default(),
        input_amount,
        base_asset_id,
        rng.gen(),
        Default::default(),
    );

    let consensus_params = ConsensusParameters::new(
        *context.get_tx_params(),
        *context.get_predicate_params(),
        *context.get_script_params(),
        *context.get_contract_params(),
        *context.get_fee_params(),
        context.get_chain_id(),
        context.get_gas_costs().to_owned(),
        *context.get_base_asset_id(),
        context.get_block_gas_limit(),
        *context.get_privileged_address(),
    );
    let create = create
        .into_checked_basic(context.get_block_height(), &consensus_params)
        .expect("failed to generate checked tx");

    let state = context.deploy(create).expect("Create should be executed");
    let change = find_change(state.tx().outputs().to_vec(), base_asset_id);

    assert_eq!(change, input_amount - spend_amount);
}

#[test]
fn change_is_reduced_by_external_transfer() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let input_amount = 1000;
    let transfer_amount: Word = 400;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();

    // simple dummy contract for transferring value to
    let contract_code = vec![op::ret(RegId::ONE)];

    let mut test_context = TestBuilder::new(2322u64);
    let contract_id = test_context
        .setup_contract(contract_code, None, None)
        .contract_id;

    // setup script for transfer
    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to contract id
            op::movi(0x10, data_offset as Immediate18),
            // set reg 0x11 to transfer amount
            op::movi(0x11, transfer_amount as Immediate18),
            // set reg 0x12 to asset id
            op::movi(0x12, (data_offset + 32) as Immediate18),
            // transfer to contract ID at 0x10, the amount of coins at 0x11, of the asset
            // id at 0x12
            op::tr(0x10, 0x11, 0x12),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let script_data = [contract_id.as_ref(), asset_id.as_ref()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // execute and get change
    let change = test_context
        .start_script(script, script_data)
        .gas_price(gas_price)
        .script_gas_limit(gas_limit)
        .coin_input(asset_id, input_amount)
        .contract_input(contract_id)
        .change_output(asset_id)
        .contract_output(&contract_id)
        .execute_get_change(asset_id);

    assert_eq!(change, input_amount - transfer_amount);
}

#[test]
fn change_is_not_reduced_by_external_transfer_on_revert() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let input_amount = 1000;
    // attempt overspend to cause a revert
    let transfer_amount: Word = input_amount + 100;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();

    // setup state for test
    // simple dummy contract for transferring value to
    let contract_code = vec![op::ret(RegId::ONE)];

    let mut test_context = TestBuilder::new(2322u64);
    let contract_id = test_context
        .setup_contract(contract_code, None, None)
        .contract_id;

    // setup script for transfer
    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to contract id
            op::movi(0x10, data_offset),
            // set reg 0x11 to transfer amount
            op::movi(0x11, transfer_amount as Immediate18),
            // set reg 0x12 to asset id
            op::movi(0x12, data_offset + 32),
            // transfer to contract ID at 0x10, the amount of coins at 0x11, of the asset
            // id at 0x12
            op::tr(0x10, 0x11, 0x12),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let script_data = [contract_id.as_ref(), asset_id.as_ref()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // execute and get change
    let change = test_context
        .start_script(script, script_data)
        .gas_price(gas_price)
        .script_gas_limit(gas_limit)
        .coin_input(asset_id, input_amount)
        .contract_input(contract_id)
        .change_output(asset_id)
        .contract_output(&contract_id)
        .execute_get_change(asset_id);

    assert_eq!(change, input_amount);
}

#[test]
fn zero_amount_transfer_reverts() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();

    // setup state for test
    // simple dummy contract for transferring value to
    let contract_code = vec![op::ret(RegId::ONE)];

    let mut test_context = TestBuilder::new(2322u64);
    let contract_id = test_context
        .setup_contract(contract_code, None, None)
        .contract_id;

    // setup script for transfer
    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to contract id
            op::movi(0x10, data_offset),
            // set reg 0x12 to asset id
            op::movi(0x11, data_offset + ContractId::LEN as u32),
            // transfer to contract id at 0x10, amount of coins is zero, asset id at 0x11
            op::tr(0x10, RegId::ZERO, 0x11),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let script_data = [contract_id.as_ref(), asset_id.as_ref()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // execute and get receipts
    let result = test_context
        .start_script(script, script_data)
        .gas_price(gas_price)
        .script_gas_limit(gas_limit)
        .coin_input(asset_id, 0)
        .contract_input(contract_id)
        .change_output(asset_id)
        .contract_output(&contract_id)
        .execute();

    let receipts = result.receipts();

    if let Some(Receipt::Panic { reason, .. }) = receipts.first() {
        assert_eq!(reason.reason(), &PanicReason::TransferZeroCoins);
    } else {
        panic!("Expected a panic receipt");
    }
}

#[test]
fn zero_amount_transfer_out_reverts() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // the initial external (coin) balance
    let external_balance = 1_000_000;
    // the amount to transfer out from external balance
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();
    let owner: Address = rng.gen();

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            // load amount of coins to 0x10
            op::movi(0x10, data_offset),
            op::lw(0x10, 0x10, 0),
            // load asset id to 0x11
            op::movi(0x11, data_offset),
            // load address to 0x12
            op::movi(0x12, data_offset + 32),
            // call contract without any tokens to transfer in or out
            op::tro(0x12, RegId::ZERO, RegId::ZERO, 0x11),
            op::ret(RegId::ONE),
        ],
        TxParameters::DEFAULT.tx_offset()
    );

    let script_data: Vec<u8> = [asset_id.as_ref(), owner.as_ref()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // execute and get receipts
    let result = TestBuilder::new(2322u64)
        .start_script(script, script_data)
        .gas_price(gas_price)
        .script_gas_limit(gas_limit)
        .coin_input(asset_id, external_balance)
        .variable_output(asset_id)
        .change_output(asset_id)
        .execute();

    let receipts = result.receipts();

    if let Some(Receipt::Panic { reason, .. }) = receipts.first() {
        assert_eq!(reason.reason(), &PanicReason::TransferZeroCoins);
    } else {
        panic!("Expected a panic receipt");
    }
}

#[test]
fn variable_output_set_by_external_transfer_out() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // the initial external (coin) balance
    let external_balance = 1_000_000;
    // the amount to transfer out from external balance
    let transfer_amount: Word = 600;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();
    let owner: Address = rng.gen();

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            // load amount of coins to 0x10
            op::movi(0x10, data_offset),
            op::lw(0x10, 0x10, 0),
            // load asset id to 0x11
            op::movi(0x11, data_offset + 8),
            // load address to 0x12
            op::movi(0x12, data_offset + 40),
            // load output index (0) to 0x13
            op::move_(0x13, RegId::ZERO),
            // call contract without any tokens to transfer in
            op::tro(0x12, 0x13, 0x10, 0x11),
            op::ret(RegId::ONE),
        ],
        TxParameters::DEFAULT.tx_offset()
    );

    let script_data: Vec<u8> = [
        transfer_amount.to_be_bytes().as_ref(),
        asset_id.as_ref(),
        owner.as_ref(),
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();

    // create and run the tx
    let result = TestBuilder::new(2322u64)
        .start_script(script, script_data)
        .gas_price(gas_price)
        .script_gas_limit(gas_limit)
        .coin_input(asset_id, external_balance)
        .variable_output(asset_id)
        .change_output(asset_id)
        .execute();

    let outputs = result.tx().outputs();
    let receipts = result.receipts();

    assert!(matches!(
        outputs[0], Output::Variable { amount, to, asset_id }
            if amount == transfer_amount
            && to == owner
            && asset_id == asset_id
    ));

    assert!(matches!(
        outputs[1], Output::Change {amount, asset_id, .. }
            if amount == external_balance - transfer_amount
            && asset_id == asset_id
    ));

    assert!(receipts
        .iter()
        .any(|r| matches!(r, Receipt::TransferOut { .. })));
}

#[test]
fn variable_output_not_set_by_external_transfer_out_on_revert() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // the initial external (coin) balance (set to less than transfer amount to
    // cause a revert)
    let external_balance = 100;
    // the amount to transfer out from external balance
    let transfer_amount: Word = 600;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();
    let owner: Address = rng.gen();
    let tx_params = TxParameters::default();

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            // load amount of coins to 0x10
            op::movi(0x10, data_offset),
            op::lw(0x10, 0x10, 0),
            // load asset id to 0x11
            op::movi(0x11, data_offset + 8),
            // load address to 0x12
            op::movi(0x12, data_offset + 40),
            // load output index (0) to 0x13
            op::move_(0x13, RegId::ZERO),
            // call contract without any tokens to transfer in
            op::tro(0x12, 0x13, 0x10, 0x11),
            op::ret(RegId::ONE),
        ],
        tx_params.tx_offset()
    );

    let script_data: Vec<u8> = [
        transfer_amount.to_be_bytes().as_ref(),
        asset_id.as_ref(),
        owner.as_ref(),
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();

    // create and run the tx
    let result = TestBuilder::new(2322u64)
        .start_script(script, script_data)
        .gas_price(gas_price)
        .script_gas_limit(gas_limit)
        .coin_input(asset_id, external_balance)
        .variable_output(asset_id)
        .change_output(asset_id)
        .execute();

    let outputs = result.tx().outputs();
    let receipts = result.receipts();

    assert!(matches!(
        outputs[0], Output::Variable { amount, .. } if amount == 0
    ));

    // full input amount is converted into change
    assert!(matches!(
        outputs[1], Output::Change {amount, asset_id, .. }
            if amount == external_balance
            && asset_id == asset_id
    ));

    // TransferOut receipt should not be present
    assert!(!receipts
        .iter()
        .any(|r| matches!(r, Receipt::TransferOut { .. })));
}

#[test]
fn variable_output_set_by_internal_contract_transfer_out() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // the initial contract balance
    let internal_balance = 1_000_000;
    // the amount to transfer out of a contract
    let transfer_amount: Word = 600;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();
    let owner: Address = rng.gen();

    // setup state for test
    let contract_code = vec![
        // load amount of coins to 0x10
        op::addi(0x10, RegId::FP, CallFrame::a_offset() as Immediate12),
        op::lw(0x10, 0x10, 0),
        // load asset id to 0x11
        op::addi(0x11, RegId::FP, CallFrame::b_offset() as Immediate12),
        op::lw(0x11, 0x11, 0),
        // load address to 0x12
        op::addi(0x12, 0x11, 32 as Immediate12),
        // load output index (0) to 0x13
        op::move_(0x13, RegId::ZERO),
        op::tro(0x12, 0x13, 0x10, 0x11),
        op::ret(RegId::ONE),
    ];
    let mut test_context = TestBuilder::new(2322u64);
    let contract_id = test_context
        .setup_contract(contract_code, Some((asset_id, internal_balance)), None)
        .contract_id;

    let (script, data_offset) = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to call data
            op::movi(0x10, (data_offset + 64) as Immediate18),
            // set reg 0x11 to transfer amount
            op::move_(0x11, RegId::CGAS),
            // call contract without any tokens to transfer in (3rd arg arbitrary when
            // 2nd is zero)
            op::call(0x10, RegId::ZERO, RegId::ZERO, 0x11),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let script_data: Vec<u8> = [
        asset_id.as_ref(),
        owner.as_ref(),
        Call::new(contract_id, transfer_amount, data_offset as Word)
            .to_bytes()
            .as_ref(),
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();

    // create and run the tx
    let result = test_context
        .start_script(script, script_data)
        .gas_price(gas_price)
        .script_gas_limit(gas_limit)
        .fee_input()
        .contract_input(contract_id)
        .variable_output(asset_id)
        .contract_output(&contract_id)
        .execute();

    let outputs = result.tx().outputs();
    let receipts = result.receipts();

    let output = Output::variable(owner, transfer_amount, asset_id);

    assert_eq!(output, outputs[0]);
    assert!(receipts
        .iter()
        .any(|r| matches!(r, Receipt::TransferOut { .. })));
}

#[test]
fn variable_output_not_increased_by_contract_transfer_out_on_revert() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // the initial contract balance (set to zero so TRO will intentionally fail)
    let internal_balance = 0;
    // the amount to transfer out of a contract
    let transfer_amount: Word = 600;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();
    let owner: Address = rng.gen();

    // setup state for test
    let contract_code = vec![
        // load amount of coins to 0x10
        op::addi(0x10, RegId::FP, CallFrame::a_offset() as Immediate12),
        op::lw(0x10, 0x10, 0),
        // load asset id to 0x11
        op::addi(0x11, RegId::FP, CallFrame::b_offset() as Immediate12),
        op::lw(0x11, 0x11, 0),
        // load to address to 0x12
        op::addi(0x12, 0x11, 32 as Immediate12),
        // load output index (0) to 0x13
        op::move_(0x13, RegId::ZERO),
        op::tro(0x12, 0x13, 0x10, 0x11),
        op::ret(RegId::ONE),
    ];

    let mut test_context = TestBuilder::new(2322u64);
    let contract_id = test_context
        .setup_contract(contract_code, Some((asset_id, internal_balance)), None)
        .contract_id;

    let (script, data_offset) = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to call data
            op::movi(0x10, data_offset + 64),
            // call contract without any tokens to transfer in
            op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let script_data: Vec<u8> = [
        asset_id.as_ref(),
        owner.as_ref(),
        Call::new(contract_id, transfer_amount, data_offset as Word)
            .to_bytes()
            .as_ref(),
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();

    // create and run the tx
    let result = test_context
        .start_script(script, script_data)
        .gas_price(gas_price)
        .script_gas_limit(gas_limit)
        .fee_input()
        .contract_input(contract_id)
        .variable_output(asset_id)
        .contract_output(&contract_id)
        .execute();

    let outputs = result.tx().outputs();
    let receipts = result.receipts();

    assert!(matches!(
        outputs[0], Output::Variable { amount, .. } if amount == 0
    ));

    // TransferOut receipt should not be present
    assert!(!receipts
        .iter()
        .any(|r| matches!(r, Receipt::TransferOut { .. })));
}

use alloc::{
    borrow::ToOwned,
    vec,
};

use crate::{
    prelude::{
        field::Outputs,
        *,
    },
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
        context.get_block_transaction_size_limit(),
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
fn correct_change_is_provided_for_data_coin_outputs_create() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    // given
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
            Output::data_coin(rng.gen(), spend_amount, base_asset_id, vec![]),
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
        context.get_block_transaction_size_limit(),
        *context.get_privileged_address(),
    );
    // when
    let create = create
        .into_checked_basic(context.get_block_height(), &consensus_params)
        .expect("failed to generate checked tx");

    // then
    let state = context.deploy(create).expect("Create should be executed");
    let change = find_change(state.tx().outputs().to_vec(), base_asset_id);

    assert_eq!(change, input_amount - spend_amount);
}

#[test]
fn transact__unverified_read_only_coin_included_but_value_not_consumed() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    // given
    let input_amount = 1000;
    let spend_amount = 600;
    let base_asset_id: AssetId = rng.gen();
    let input_owner = rng.gen();
    let utxo_id = rng.gen();
    let read_only_input = Input::read_only_unverified_coin(
        utxo_id,
        Address::default(),
        input_amount.clone(),
        AssetId::default(),
        TxPointer::default(),
    );

    let change_output = Output::change(rng.gen(), 0, base_asset_id);
    let mut script = Transaction::script(
        10000,
        vec![],
        vec![],
        Policies::new().with_max_fee(0),
        vec![],
        vec![change_output],
        vec![Witness::default()],
    );
    script.add_unsigned_coin_input(
        utxo_id,
        &Default::default(),
        spend_amount,
        base_asset_id,
        input_owner,
        Default::default(),
    );

    script.add_unverified_read_only_coin_input(
        rng.gen(),
        &Default::default(),
        input_amount,
        base_asset_id,
        rng.gen(),
    );

    let mut context = TestBuilder::new(2322u64);
    let context = context.base_asset_id(base_asset_id);

    let consensus_params = context.consensus_params();

    // when
    let checked = script
        .into_checked_basic(context.get_block_height(), &consensus_params)
        .expect("failed to generate checked tx");

    // then
    let state = context
        .execute_tx(checked)
        .expect("Create should be executed");
    let change = find_change(state.tx().outputs().to_vec(), base_asset_id);

    assert_eq!(change, spend_amount);
}

#[test]
fn transact__unverified_read_only_data_coin_included_but_value_not_consumed() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    // given
    let input_amount = 1000;
    let spend_amount = 600;
    let base_asset_id: AssetId = rng.gen();
    let input_owner = rng.gen();
    let utxo_id = rng.gen();
    let read_only_input = Input::read_only_unverified_coin(
        utxo_id,
        Address::default(),
        input_amount.clone(),
        AssetId::default(),
        TxPointer::default(),
    );

    let change_output = Output::change(rng.gen(), 0, base_asset_id);
    let mut script = Transaction::script(
        10000,
        vec![],
        vec![],
        Policies::new().with_max_fee(0),
        vec![],
        vec![change_output],
        vec![Witness::default()],
    );
    script.add_unsigned_coin_input(
        utxo_id,
        &Default::default(),
        spend_amount,
        base_asset_id,
        input_owner,
        Default::default(),
    );

    let data = vec![1, 2, 3, 4, 5, 6];
    script.add_unverified_read_only_data_coin_input(
        rng.gen(),
        &Default::default(),
        input_amount,
        base_asset_id,
        rng.gen(),
        data,
    );

    let mut context = TestBuilder::new(2322u64);
    let context = context.base_asset_id(base_asset_id);

    let consensus_params = context.consensus_params();

    // when
    let checked = script
        .into_checked_basic(context.get_block_height(), &consensus_params)
        .expect("failed to generate checked tx");

    // then
    let state = context
        .execute_tx(checked)
        .expect("Create should be executed");
    let change = find_change(state.tx().outputs().to_vec(), base_asset_id);

    assert_eq!(change, spend_amount);
}

#[test]
fn transact__verified_read_only_coin_included_but_value_not_consumed() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    // given
    let input_amount = 1000;
    let spend_amount = 600;
    let base_asset_id: AssetId = rng.gen();
    let input_owner = rng.gen();
    let utxo_id = rng.gen();
    let read_only_input = Input::read_only_unverified_coin(
        utxo_id,
        Address::default(),
        input_amount.clone(),
        AssetId::default(),
        TxPointer::default(),
    );

    let change_output = Output::change(rng.gen(), 0, base_asset_id);
    let mut script = Transaction::script(
        10000,
        vec![],
        vec![],
        Policies::new().with_max_fee(0),
        vec![],
        vec![change_output],
        vec![Witness::default()],
    );
    script.add_unsigned_coin_input(
        utxo_id,
        &Default::default(),
        spend_amount,
        base_asset_id,
        input_owner,
        Default::default(),
    );

    let true_predicate = vec![op::ret(0x01)].into_iter().collect();
    script.add_verified_read_only_coin_input(
        rng.gen(),
        &Default::default(),
        input_amount,
        base_asset_id,
        rng.gen(),
        0,
        true_predicate,
        vec![],
    );

    let mut context = TestBuilder::new(2322u64);
    let context = context.base_asset_id(base_asset_id);

    let consensus_params = context.consensus_params();

    // when
    let checked = script
        .into_checked_basic(context.get_block_height(), &consensus_params)
        .expect("failed to generate checked tx");

    // then
    let state = context
        .execute_tx(checked)
        .expect("Create should be executed");
    let change = find_change(state.tx().outputs().to_vec(), base_asset_id);

    assert_eq!(change, spend_amount);
}

#[test]
fn transact__verified_read_only_data_coin_included_but_value_not_consumed() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    // given
    let input_amount = 1000;
    let spend_amount = 600;
    let base_asset_id: AssetId = rng.gen();
    let input_owner = rng.gen();
    let utxo_id = rng.gen();
    let read_only_input = Input::read_only_unverified_coin(
        utxo_id,
        Address::default(),
        input_amount.clone(),
        AssetId::default(),
        TxPointer::default(),
    );

    let change_output = Output::change(rng.gen(), 0, base_asset_id);
    let mut script = Transaction::script(
        10000,
        vec![],
        vec![],
        Policies::new().with_max_fee(0),
        vec![],
        vec![change_output],
        vec![Witness::default()],
    );
    script.add_unsigned_coin_input(
        utxo_id,
        &Default::default(),
        spend_amount,
        base_asset_id,
        input_owner,
        Default::default(),
    );

    let data = vec![1, 2, 3, 4, 5, 6];
    let true_predicate = vec![op::ret(0x01)].into_iter().collect();
    script.add_verified_read_only_data_coin_input(
        rng.gen(),
        &Default::default(),
        input_amount,
        base_asset_id,
        rng.gen(),
        0,
        true_predicate,
        vec![],
        data,
    );

    let mut context = TestBuilder::new(2322u64);
    let context = context.base_asset_id(base_asset_id);

    let consensus_params = context.consensus_params();

    // when
    let checked = script
        .into_checked_basic(context.get_block_height(), &consensus_params)
        .expect("failed to generate checked tx");

    // then
    let state = context
        .execute_tx(checked)
        .expect("Create should be executed");
    let change = find_change(state.tx().outputs().to_vec(), base_asset_id);

    assert_eq!(change, spend_amount);
}

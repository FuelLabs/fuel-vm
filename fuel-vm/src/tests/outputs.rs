use alloc::{
    borrow::ToOwned,
    vec,
};

use crate::{
    interpreter::{
        CheckedMetadata,
        RuntimeBalances,
    },
    prelude::{
        field::Outputs,
        *,
    },
    util::test_helpers::{
        TestBuilder,
        find_change,
    },
};
use fuel_asm::{
    RegId,
    op,
};
use fuel_tx::{
    ConsensusParameters,
    Witness,
    policies::Policies,
};
use rand::{
    Rng,
    SeedableRng,
    rngs::StdRng,
};

/// Testing of post-execution output handling
#[test]
fn full_change_with_no_fees() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let input_amount = 1000;
    let gas_price = 0;
    let base_asset_id: AssetId = rng.r#gen();

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
    let base_asset_id = rng.r#gen();
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
    let base_asset_id = rng.r#gen();
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
    let asset_id: AssetId = rng.r#gen();

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .coin_input(asset_id, input_amount)
        .change_output(asset_id)
        .coin_output(asset_id, spend_amount)
        .execute_get_change(asset_id);

    assert_eq!(change, input_amount - spend_amount);
}

#[rstest::rstest]
fn change_with_retryable_base_and_separate_fee_asset(
    #[values(false, true)] revert: bool,
) {
    let rng = &mut StdRng::seed_from_u64(2322);
    let key = SecretKey::random(rng);
    let fee_asset = AssetId::new([7; 32]);
    let (mut tx, metadata) = TransactionBuilder::script(vec![], vec![])
        .add_unsigned_coin_input(key, rng.r#gen(), 10, fee_asset, rng.r#gen())
        .add_unsigned_message_input(key, rng.r#gen(), rng.r#gen(), 8, vec![1])
        .add_output(Output::change(Default::default(), 0, fee_asset))
        .add_output(Output::change(Default::default(), 0, AssetId::BASE))
        .fee_asset(0)
        .max_fee_limit(4)
        .tip(1)
        .finalize_checked(Default::default())
        .into();

    let initial = metadata.balances();
    let balances = RuntimeBalances::try_from(initial.clone()).unwrap();
    let params = ConsensusParameters::standard();
    tx.update_outputs(
        revert,
        0,
        &initial,
        &balances,
        params.gas_costs(),
        params.fee_params(),
        params.base_asset_id(),
        0,
    )
    .unwrap();

    assert_eq!(tx.outputs()[0].amount(), Some(9));
    assert_eq!(tx.outputs()[1].amount(), Some(if revert { 0 } else { 8 }));
}

#[test]
fn correct_change_is_provided_for_coin_outputs_create() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let input_amount = 1000;
    let spend_amount = 600;
    let base_asset_id: AssetId = rng.r#gen();

    #[rustfmt::skip]
    let invalid_instruction_bytecode = vec![0u8; 4];

    let salt: Salt = rng.r#gen();
    let program: Witness = invalid_instruction_bytecode.into();

    let contract_root = Contract::root_from_code(program.as_ref());
    let state_root = Contract::default_state_root();
    let contract_undefined = Contract::id(&salt, &contract_root, &state_root);

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
            Output::change(rng.r#gen(), 0, base_asset_id),
            Output::coin(rng.r#gen(), spend_amount, base_asset_id),
        ],
        vec![program, Witness::default()],
    );
    create.add_unsigned_coin_input(
        rng.r#gen(),
        &Default::default(),
        input_amount,
        base_asset_id,
        rng.r#gen(),
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

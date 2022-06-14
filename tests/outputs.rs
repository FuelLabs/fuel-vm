use fuel_vm::consts::REG_CGAS;
use fuel_vm::{
    consts::{REG_FP, REG_ONE, REG_ZERO},
    prelude::*,
    script_with_data_offset,
    util::test_helpers::TestBuilder,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Testing of post-execution output handling

#[test]
fn full_change_with_no_fees() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .execute_get_change(AssetId::default());

    assert_eq!(change, input_amount);
}

#[test]
fn byte_fees_are_deducted_from_base_asset_change() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 1;

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .execute_get_change(AssetId::default());

    assert!(change < input_amount);
}

#[test]
fn used_gas_is_deducted_from_base_asset_change() {
    let input_amount = 1000;
    let gas_price = 1;
    let byte_price = 0;

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .execute_get_change(AssetId::default());

    assert!(change < input_amount);
}

#[test]
fn used_gas_is_deducted_from_base_asset_change_on_revert() {
    let input_amount = 1000;
    let gas_price = 1;
    let byte_price = 0;

    let change = TestBuilder::new(2322u64)
        .script(vec![
            // Log some dummy data to burn extra gas
            Opcode::LOG(REG_ONE, REG_ONE, REG_ONE, REG_ONE),
            // Revert transaction
            Opcode::RVRT(REG_ONE),
        ])
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(AssetId::default(), input_amount)
        .change_output(AssetId::default())
        .execute_get_change(AssetId::default());

    assert!(change < input_amount);
}

#[test]
fn correct_change_is_provided_for_coin_outputs() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;
    let spend_amount = 600;
    let asset_id = AssetId::default();

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(asset_id, input_amount)
        .change_output(asset_id)
        .coin_output(asset_id, spend_amount)
        .execute_get_change(asset_id);

    assert_eq!(change, input_amount - spend_amount);
}

#[test]
fn correct_change_is_provided_for_withdrawal_outputs() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;
    let spend_amount = 650;
    let asset_id = AssetId::default();

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(asset_id, input_amount)
        .change_output(asset_id)
        .withdrawal_output(asset_id, spend_amount)
        .execute_get_change(asset_id);

    assert_eq!(change, input_amount - spend_amount);
}

#[test]
fn change_is_reduced_by_external_transfer() {
    let input_amount = 1000;
    let transfer_amount: Word = 400;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let asset_id = AssetId::default();

    // simple dummy contract for transferring value to
    let contract_code = vec![Opcode::RET(REG_ONE)];

    let mut test_context = TestBuilder::new(2322u64);
    let contract_id = test_context.setup_contract(contract_code, None, None).contract_id;

    // setup script for transfer
    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to contract id
            Opcode::MOVI(0x10, data_offset as Immediate18),
            // set reg 0x11 to transfer amount
            Opcode::MOVI(0x11, transfer_amount as Immediate18),
            // set reg 0x12 to asset id
            Opcode::MOVI(0x12, (data_offset + 32) as Immediate18),
            // transfer to contract ID at 0x10, the amount of coins at 0x11, of the asset id at 0x12
            Opcode::TR(0x10, 0x11, 0x12),
            Opcode::RET(REG_ONE),
        ],
        test_context.tx_offset()
    );

    let script_data = [contract_id.as_ref(), asset_id.as_ref()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // execute and get change
    let change = test_context
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .coin_input(asset_id, input_amount)
        .contract_input(contract_id)
        .change_output(asset_id)
        .contract_output(&contract_id)
        .script(script)
        .script_data(script_data)
        .execute_get_change(asset_id);

    assert_eq!(change, input_amount - transfer_amount);
}

#[test]
fn change_is_not_reduced_by_external_transfer_on_revert() {
    let input_amount = 1000;
    // attempt overspend to cause a revert
    let transfer_amount: Word = input_amount + 100;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let asset_id = AssetId::default();

    // setup state for test
    // simple dummy contract for transferring value to
    let contract_code = vec![Opcode::RET(REG_ONE)];

    let mut test_context = TestBuilder::new(2322u64);
    let contract_id = test_context.setup_contract(contract_code, None, None).contract_id;

    // setup script for transfer
    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to contract id
            Opcode::MOVI(0x10, data_offset),
            // set reg 0x11 to transfer amount
            Opcode::MOVI(0x11, transfer_amount as Immediate18),
            // set reg 0x12 to asset id
            Opcode::MOVI(0x12, data_offset + 32),
            // transfer to contract ID at 0x10, the amount of coins at 0x11, of the asset id at 0x12
            Opcode::TR(0x10, 0x11, 0x12),
            Opcode::RET(REG_ONE),
        ],
        test_context.tx_offset()
    );

    let script_data = [contract_id.as_ref(), asset_id.as_ref()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // execute and get change
    let change = test_context
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .coin_input(asset_id, input_amount)
        .contract_input(contract_id)
        .change_output(asset_id)
        .contract_output(&contract_id)
        .script(script)
        .script_data(script_data)
        .execute_get_change(asset_id);

    assert_eq!(change, input_amount);
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
    let byte_price = 0;
    let asset_id = AssetId::default();
    let owner: Address = rng.gen();

    let params = ConsensusParameters::default();

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            // load amount of coins to 0x10
            Opcode::MOVI(0x10, data_offset),
            Opcode::LW(0x10, 0x10, 0),
            // load asset id to 0x11
            Opcode::MOVI(0x11, data_offset + 8),
            // load address to 0x12
            Opcode::MOVI(0x12, data_offset + 40),
            // load output index (0) to 0x13
            Opcode::MOVE(0x13, REG_ZERO),
            // call contract without any tokens to transfer in
            Opcode::TRO(0x12, 0x13, 0x10, 0x11),
            Opcode::RET(REG_ONE),
        ],
        params.tx_offset()
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
    let outputs = TestBuilder::new(2322u64)
        .params(params)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .coin_input(asset_id, external_balance)
        .variable_output(asset_id)
        .change_output(asset_id)
        .script(script)
        .script_data(script_data)
        .execute_get_outputs();

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
    let byte_price = 0;
    let asset_id = AssetId::default();
    let owner: Address = rng.gen();

    let params = ConsensusParameters::default();

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            // load amount of coins to 0x10
            Opcode::MOVI(0x10, data_offset),
            Opcode::LW(0x10, 0x10, 0),
            // load asset id to 0x11
            Opcode::MOVI(0x11, data_offset + 8),
            // load address to 0x12
            Opcode::MOVI(0x12, data_offset + 40),
            // load output index (0) to 0x13
            Opcode::MOVE(0x13, REG_ZERO),
            // call contract without any tokens to transfer in
            Opcode::TRO(0x12, 0x13, 0x10, 0x11),
            Opcode::RET(REG_ONE),
        ],
        params.tx_offset()
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
    let outputs = TestBuilder::new(2322u64)
        .params(params)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .coin_input(asset_id, external_balance)
        .variable_output(asset_id)
        .change_output(asset_id)
        .script(script)
        .script_data(script_data)
        .execute_get_outputs();

    assert!(matches!(
        outputs[0], Output::Variable { amount, .. } if amount == 0
    ));

    // full input amount is converted into change
    assert!(matches!(
        outputs[1], Output::Change {amount, asset_id, .. }
            if amount == external_balance
            && asset_id == asset_id
    ));
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
    let byte_price = 0;
    let asset_id = AssetId::default();
    let owner: Address = rng.gen();

    // setup state for test
    let contract_code = vec![
        // load amount of coins to 0x10
        Opcode::ADDI(0x10, REG_FP, CallFrame::a_offset() as Immediate12),
        Opcode::LW(0x10, 0x10, 0),
        // load asset id to 0x11
        Opcode::ADDI(0x11, REG_FP, CallFrame::b_offset() as Immediate12),
        Opcode::LW(0x11, 0x11, 0),
        // load address to 0x12
        Opcode::ADDI(0x12, 0x11, 32 as Immediate12),
        // load output index (0) to 0x13
        Opcode::MOVE(0x13, REG_ZERO),
        Opcode::TRO(0x12, 0x13, 0x10, 0x11),
        Opcode::RET(REG_ONE),
    ];
    let mut test_context = TestBuilder::new(2322u64);
    let contract_id = test_context
        .setup_contract(contract_code, Some((asset_id, internal_balance)), None)
        .contract_id;

    let (script, data_offset) = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to call data
            Opcode::MOVI(0x10, (data_offset + 64) as Immediate18),
            // set reg 0x11 to transfer amount
            Opcode::MOVE(0x11, REG_CGAS),
            // call contract without any tokens to transfer in (3rd arg arbitrary when 2nd is zero)
            Opcode::CALL(0x10, REG_ZERO, REG_ZERO, 0x11),
            Opcode::RET(REG_ONE),
        ],
        test_context.tx_offset()
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
    let outputs = test_context
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .contract_input(contract_id)
        .variable_output(asset_id)
        .contract_output(&contract_id)
        .script(script)
        .script_data(script_data)
        .execute_get_outputs();

    assert!(matches!(
        outputs[0], Output::Variable { amount, to, asset_id }
            if amount == transfer_amount
            && to == owner
            && asset_id == asset_id
    ));
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
    let byte_price = 0;
    let asset_id = AssetId::default();
    let owner: Address = rng.gen();

    // setup state for test
    let contract_code = vec![
        // load amount of coins to 0x10
        Opcode::ADDI(0x10, REG_FP, CallFrame::a_offset() as Immediate12),
        Opcode::LW(0x10, 0x10, 0),
        // load asset id to 0x11
        Opcode::ADDI(0x11, REG_FP, CallFrame::b_offset() as Immediate12),
        Opcode::LW(0x11, 0x11, 0),
        // load to address to 0x12
        Opcode::ADDI(0x12, 0x11, 32 as Immediate12),
        // load output index (0) to 0x13
        Opcode::MOVE(0x13, REG_ZERO),
        Opcode::TRO(0x12, 0x13, 0x10, 0x11),
        Opcode::RET(REG_ONE),
    ];

    let mut test_context = TestBuilder::new(2322u64);
    let contract_id = test_context
        .setup_contract(contract_code, Some((asset_id, internal_balance)), None)
        .contract_id;

    let (script, data_offset) = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to call data
            Opcode::MOVI(0x10, data_offset + 64),
            // call contract without any tokens to transfer in
            Opcode::CALL(0x10, REG_ZERO, REG_ZERO, REG_CGAS),
            Opcode::RET(REG_ONE),
        ],
        test_context.tx_offset()
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
    let outputs = test_context
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .contract_input(contract_id)
        .variable_output(asset_id)
        .contract_output(&contract_id)
        .script(script)
        .script_data(script_data)
        .execute_get_outputs();

    assert!(matches!(
        outputs[0], Output::Variable { amount, .. } if amount == 0
    ));
}

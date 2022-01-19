use fuel_types::bytes;
use fuel_vm::consts::{REG_FP, VM_TX_MEMORY};
use fuel_vm::util::test_helpers::TestBuilder;
use fuel_vm::{
    consts::{REG_ONE, REG_ZERO},
    prelude::*,
    script_with_data_offset,
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
        .coin_input(Color::default(), input_amount)
        .change_output(Color::default())
        .execute_get_change(Color::default());

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
        .coin_input(Color::default(), input_amount)
        .change_output(Color::default())
        .execute_get_change(Color::default());

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
        .coin_input(Color::default(), input_amount)
        .change_output(Color::default())
        .execute_get_change(Color::default());

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
        .coin_input(Color::default(), input_amount)
        .change_output(Color::default())
        .execute_get_change(Color::default());

    assert!(change < input_amount);
}

#[test]
fn correct_change_is_provided_for_coin_outputs() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;
    let spend_amount = 600;
    let color = Color::default();

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(color, input_amount)
        .change_output(color)
        .coin_output(color, spend_amount)
        .execute_get_change(color);

    assert_eq!(change, input_amount - spend_amount);
}

#[test]
fn correct_change_is_provided_for_withdrawal_outputs() {
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;
    let spend_amount = 650;
    let color = Color::default();

    let change = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(color, input_amount)
        .change_output(color)
        .withdrawal_output(color, spend_amount)
        .execute_get_change(color);

    assert_eq!(change, input_amount - spend_amount);
}

#[test]
#[should_panic(expected = "ValidationError(TransactionOutputChangeColorDuplicated)")]
fn change_is_not_duplicated_for_each_base_asset_change_output() {
    // create multiple change outputs for the base asset and ensure the total change is correct
    let input_amount = 1000;
    let gas_price = 0;
    let byte_price = 0;
    let color = Color::default();

    let outputs = TestBuilder::new(2322u64)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .coin_input(color, input_amount)
        .change_output(color)
        .change_output(color)
        .execute();

    let mut total_change = 0;
    for output in outputs {
        if let Output::Change { amount, .. } = output {
            total_change += amount;
        }
    }
    // verify total change matches the input amount
    assert_eq!(total_change, input_amount);
}

#[test]
fn change_is_reduced_by_external_transfer() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let input_amount = 1000;
    let transfer_amount: Word = 400;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let asset_id = Color::default();

    // setup state for test
    let mut storage = MemoryStorage::default();
    // simple dummy contract for transferring value to
    let contract_code = vec![Opcode::RET(REG_ONE)].into_iter().collect::<Vec<u8>>();
    let contract = Contract::from(contract_code.as_ref());
    let salt: Salt = rng.gen();
    let contract_root = contract.root();
    let contract_id = contract.id(&salt, &contract_root);
    storage.storage_contract_insert(&contract_id, &contract).unwrap();
    storage
        .storage_contract_root_insert(&contract_id, &salt, &contract_root)
        .unwrap();

    // setup script for transfer
    let script = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to contract id
            Opcode::ADDI(0x10, REG_ZERO, data_offset as Immediate12),
            // set reg 0x11 to transfer amount
            Opcode::ADDI(0x11, REG_ZERO, transfer_amount as Immediate12),
            // set reg 0x12 to color
            Opcode::ADDI(0x12, REG_ZERO, (data_offset + 32) as Immediate12),
            // transfer to contract ID at 0x10, the amount of coins at 0x11, of the color at 0x12
            Opcode::TR(0x10, 0x11, 0x12),
            Opcode::RET(REG_ONE),
        ]
    );

    let script_data = [contract_id.as_ref(), asset_id.as_ref()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // execute and get change
    let change = TestBuilder::new(2322u64)
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
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let input_amount = 1000;
    // attempt overspend to cause a revert
    let transfer_amount: Word = input_amount + 100;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let asset_id = Color::default();

    // setup state for test
    let mut storage = MemoryStorage::default();
    // simple dummy contract for transferring value to
    let contract_code = vec![Opcode::RET(REG_ONE)].into_iter().collect::<Vec<u8>>();
    let contract = Contract::from(contract_code.as_ref());
    let salt: Salt = rng.gen();
    let contract_root = contract.root();
    let contract_id = contract.id(&salt, &contract_root);
    storage.storage_contract_insert(&contract_id, &contract).unwrap();
    storage
        .storage_contract_root_insert(&contract_id, &salt, &contract_root)
        .unwrap();

    // setup script for transfer
    let script = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to contract id
            Opcode::ADDI(0x10, REG_ZERO, data_offset as Immediate12),
            // set reg 0x11 to transfer amount
            Opcode::ADDI(0x11, REG_ZERO, transfer_amount as Immediate12),
            // set reg 0x12 to color
            Opcode::ADDI(0x12, REG_ZERO, (data_offset + 32) as Immediate12),
            // transfer to contract ID at 0x10, the amount of coins at 0x11, of the color at 0x12
            Opcode::TR(0x10, 0x11, 0x12),
            Opcode::RET(REG_ONE),
        ]
    );

    let script_data = [contract_id.as_ref(), asset_id.as_ref()]
        .into_iter()
        .flatten()
        .copied()
        .collect();

    // execute and get change
    let change = TestBuilder::new(2322u64)
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
    let asset_id = Color::default();
    let owner: Address = rng.gen();

    let script = script_with_data_offset!(
        data_offset,
        vec![
            // load amount of coins to 0x10
            Opcode::ADDI(0x10, REG_ZERO, data_offset),
            Opcode::LW(0x10, 0x10, 0),
            // load color to 0x11
            Opcode::ADDI(0x11, REG_ZERO, data_offset + 8),
            // load address to 0x12
            Opcode::ADDI(0x12, REG_ZERO, data_offset + 40),
            // load output index (0) to 0x13
            Opcode::ADDI(0x13, REG_ZERO, 0),
            // call contract without any tokens to transfer in
            Opcode::TRO(0x12, 0x13, 0x10, 0x11),
            Opcode::RET(REG_ONE),
        ]
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
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .coin_input(asset_id, external_balance)
        .variable_output(asset_id)
        .change_output(asset_id)
        .script(script)
        .script_data(script_data)
        .execute();

    assert!(matches!(
        outputs[0], Output::Variable { amount, to, color }
            if amount == transfer_amount
            && to == owner
            && color == asset_id
    ));

    assert!(matches!(
        outputs[1], Output::Change {amount, color, .. }
            if amount == external_balance - transfer_amount
            && color == asset_id
    ));
}

#[test]
fn variable_output_not_set_by_external_transfer_out_on_revert() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // the initial external (coin) balance (set to less than transfer amount to cause a revert)
    let external_balance = 100;
    // the amount to transfer out from external balance
    let transfer_amount: Word = 600;
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let asset_id = Color::default();
    let owner: Address = rng.gen();

    let script = script_with_data_offset!(
        data_offset,
        vec![
            // load amount of coins to 0x10
            Opcode::ADDI(0x10, REG_ZERO, data_offset),
            Opcode::LW(0x10, 0x10, 0),
            // load color to 0x11
            Opcode::ADDI(0x11, REG_ZERO, data_offset + 8),
            // load address to 0x12
            Opcode::ADDI(0x12, REG_ZERO, data_offset + 40),
            // load output index (0) to 0x13
            Opcode::ADDI(0x13, REG_ZERO, 0),
            // call contract without any tokens to transfer in
            Opcode::TRO(0x12, 0x13, 0x10, 0x11),
            Opcode::RET(REG_ONE),
        ]
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
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .coin_input(asset_id, external_balance)
        .variable_output(asset_id)
        .change_output(asset_id)
        .script(script)
        .script_data(script_data)
        .execute();

    assert!(matches!(
        outputs[0], Output::Variable { amount, .. } if amount == 0
    ));

    // full input amount is converted into change
    assert!(matches!(
        outputs[1], Output::Change {amount, color, .. }
            if amount == external_balance
            && color == asset_id
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
    let asset_id = Color::default();
    let owner: Address = rng.gen();

    // setup state for test
    let mut storage = MemoryStorage::default();
    let contract_code: Witness = [
        // load amount of coins to 0x10
        Opcode::ADDI(0x10, REG_FP, CallFrame::a_offset() as Immediate12),
        Opcode::LW(0x10, 0x10, 0),
        // load color to 0x11
        Opcode::ADDI(0x11, REG_FP, CallFrame::b_offset() as Immediate12),
        Opcode::LW(0x11, 0x11, 0),
        // load address to 0x12
        Opcode::ADDI(0x12, 0x11, 32 as Immediate12),
        // load output index (0) to 0x13
        Opcode::ADDI(0x13, REG_ZERO, 0 as Immediate12),
        Opcode::TRO(0x12, 0x13, 0x10, 0x11),
        Opcode::RET(REG_ONE),
    ]
    .iter()
    .copied()
    .collect::<Vec<u8>>()
    .into();
    let contract = Contract::from(contract_code.as_ref());
    let salt: Salt = rng.gen();
    let contract_root = contract.root();
    let contract_id = contract.id(&salt, &contract_root);
    storage.storage_contract_insert(&contract_id, &contract).unwrap();
    storage
        .storage_contract_root_insert(&contract_id, &salt, &contract_root)
        .unwrap();
    // set internal contract balance
    storage
        .merkle_contract_color_balance_insert(&contract_id, &asset_id, internal_balance)
        .unwrap();

    let script = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to call data
            Opcode::ADDI(0x10, REG_ZERO, (data_offset + 64) as Immediate12),
            // set reg 0x11 to transfer amount
            Opcode::ADDI(0x11, REG_ZERO, gas_limit as Immediate12),
            // call contract without any tokens to transfer in
            Opcode::CALL(0x10, REG_ZERO, 0x10, 0x11),
            Opcode::RET(REG_ONE),
        ]
    );

    let script_bytes: Vec<u8> = script.clone().into_iter().collect();
    let data_offset = VM_TX_MEMORY + Transaction::script_offset() + bytes::padded_len(script_bytes.as_slice());
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
    let outputs = TestBuilder::new(2322u64)
        .storage(storage)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .contract_input(contract_id)
        .variable_output(asset_id)
        .contract_output(&contract_id)
        .script(script)
        .script_data(script_data)
        .execute();

    assert!(matches!(
        outputs[0], Output::Variable { amount, to, color }
            if amount == transfer_amount
            && to == owner
            && color == asset_id
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
    let asset_id = Color::default();
    let owner: Address = rng.gen();

    // setup state for test
    let mut storage = MemoryStorage::default();
    let contract_code: Witness = [
        // load amount of coins to 0x10
        Opcode::ADDI(0x10, REG_FP, CallFrame::a_offset() as Immediate12),
        Opcode::LW(0x10, 0x10, 0),
        // load color to 0x11
        Opcode::ADDI(0x11, REG_FP, CallFrame::b_offset() as Immediate12),
        Opcode::LW(0x11, 0x11, 0),
        // load to address to 0x12
        Opcode::ADDI(0x12, 0x11, 32 as Immediate12),
        // load output index (0) to 0x13
        Opcode::ADDI(0x13, REG_ZERO, 0 as Immediate12),
        Opcode::TRO(0x12, 0x13, 0x10, 0x11),
        Opcode::RET(REG_ONE),
    ]
    .iter()
    .copied()
    .collect::<Vec<u8>>()
    .into();
    let contract = Contract::from(contract_code.as_ref());
    let salt: Salt = rng.gen();
    let contract_root = contract.root();
    let contract_id = contract.id(&salt, &contract_root);
    storage.storage_contract_insert(&contract_id, &contract).unwrap();
    storage
        .storage_contract_root_insert(&contract_id, &salt, &contract_root)
        .unwrap();
    // set internal contract balance
    storage
        .merkle_contract_color_balance_insert(&contract_id, &asset_id, internal_balance)
        .unwrap();

    let script = script_with_data_offset!(
        data_offset,
        vec![
            // set reg 0x10 to call data
            Opcode::ADDI(0x10, REG_ZERO, (data_offset + 64) as Immediate12),
            // set reg 0x11 to transfer amount
            Opcode::ADDI(0x11, REG_ZERO, gas_limit as Immediate12),
            // call contract without any tokens to transfer in
            Opcode::CALL(0x10, REG_ZERO, 0x10, 0x11),
            Opcode::RET(REG_ONE),
        ]
    );

    let script_bytes: Vec<u8> = script.clone().into_iter().collect();
    let data_offset = VM_TX_MEMORY + Transaction::script_offset() + bytes::padded_len(script_bytes.as_slice());
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
    let outputs = TestBuilder::new(2322u64)
        .storage(storage)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .byte_price(byte_price)
        .contract_input(contract_id)
        .variable_output(asset_id)
        .contract_output(&contract_id)
        .script(script)
        .script_data(script_data)
        .execute();

    assert!(matches!(
        outputs[0], Output::Variable { amount, .. } if amount == 0
    ));
}

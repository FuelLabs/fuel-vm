use fuel_vm::consts::REG_ONE;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Testing of post-execution output handling

#[test]
fn full_change_with_no_fees() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 100;
    let byte_price = 0;
    let maturity = 0;
    let input_amount = 1000;
    let owner = rng.gen();

    let input = Input::coin(
        rng.gen(),
        owner,
        input_amount,
        Color::default(),
        0,
        maturity,
        vec![],
        vec![],
    );

    let output = Output::change(owner, 0, Color::default());

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        vec![Opcode::RET(REG_ONE)].into_iter().collect(),
        vec![],
        vec![input],
        vec![output],
        vec![Witness::default()],
    );

    client.transact(tx);

    let txtor: &Transactor<_> = client.as_ref();
    let outputs = txtor.state_transition().unwrap().tx().outputs();
    let output = outputs.first().unwrap();
    assert!(
        matches!(output, Output::Change {amount, color, ..} if amount == &input_amount && color == &Color::default())
    );
}

#[test]
fn byte_fees_are_deducted_from_base_asset_change() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 100;
    let byte_price = 1;
    let maturity = 0;
    let input_amount = 1000;
    let owner = rng.gen();

    let input = Input::coin(
        rng.gen(),
        owner,
        input_amount,
        Color::default(),
        0,
        maturity,
        vec![],
        vec![],
    );

    let output = Output::change(owner, 0, Color::default());

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        vec![Opcode::RET(REG_ONE)].into_iter().collect(),
        vec![],
        vec![input],
        vec![output],
        vec![Witness::default()],
    );

    client.transact(tx);

    let txtor: &Transactor<_> = client.as_ref();
    let outputs = txtor.state_transition().unwrap().tx().outputs();
    let output = outputs.first().unwrap();
    assert!(
        matches!(output, Output::Change {amount, color, ..} if amount < &input_amount && color == &Color::default())
    );
}

#[test]
fn used_gas_is_deducted_from_base_asset_change() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 1;
    let gas_limit = 100;
    let byte_price = 0;
    let maturity = 0;
    let input_amount = 1000;
    let owner = rng.gen();

    let input = Input::coin(
        rng.gen(),
        owner,
        input_amount,
        Color::default(),
        0,
        maturity,
        vec![],
        vec![],
    );

    let output = Output::change(owner, 0, Color::default());

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        vec![Opcode::RET(REG_ONE)].into_iter().collect(),
        vec![],
        vec![input],
        vec![output],
        vec![Witness::default()],
    );

    client.transact(tx);

    let txtor: &Transactor<_> = client.as_ref();
    let outputs = txtor.state_transition().unwrap().tx().outputs();
    let output = outputs.first().unwrap();
    assert!(
        matches!(output, Output::Change {amount, color, ..} if amount < &input_amount && color == &Color::default())
    );
}

#[test]
fn correct_change_is_provided_for_coin_outputs() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 100;
    let byte_price = 0;
    let maturity = 0;
    let input_amount = 1000;
    let spend_amount = 600;
    let owner = rng.gen();
    let color = Color::default();

    let input = Input::coin(rng.gen(), owner, input_amount, color, 0, maturity, vec![], vec![]);

    let change_output = Output::change(owner, 0, color);
    let coin_output = Output::coin(rng.gen(), spend_amount, color);

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        vec![Opcode::RET(REG_ONE)].into_iter().collect(),
        vec![],
        vec![input],
        vec![change_output, coin_output],
        vec![Witness::default()],
    );

    client.transact(tx);

    let txtor: &Transactor<_> = client.as_ref();
    let outputs = txtor.state_transition().unwrap().tx().outputs();
    let output = outputs.first().unwrap();
    assert!(
        matches!(output, Output::Change {amount, color, ..} if amount == &(input_amount - spend_amount) && color == &Color::default())
    );
}

#[test]
fn correct_change_is_provided_for_withdrawal_outputs() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 100;
    let byte_price = 0;
    let maturity = 0;
    let input_amount = 1000;
    let spend_amount = 650;
    let owner = rng.gen();
    let color = Color::default();

    let input = Input::coin(rng.gen(), owner, input_amount, color, 0, maturity, vec![], vec![]);

    let change_output = Output::change(owner, 0, color);
    let withdraw_output = Output::withdrawal(rng.gen(), spend_amount, color);

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        vec![Opcode::RET(REG_ONE)].into_iter().collect(),
        vec![],
        vec![input],
        vec![change_output, withdraw_output],
        vec![Witness::default()],
    );

    client.transact(tx);

    let txtor: &Transactor<_> = client.as_ref();
    let outputs = txtor.state_transition().unwrap().tx().outputs();
    let output = outputs.first().unwrap();
    assert!(
        matches!(output, Output::Change {amount, color, ..} if amount == &(input_amount - spend_amount) && color == &Color::default())
    );
}

//TOOD: fix in fuel-tx validation
#[test]
#[ignore]
fn change_isnt_duplicated_for_each_base_asset_change_output() {
    // create multiple change outputs for the base asset and ensure the total change is correct
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 100;
    let byte_price = 0;
    let maturity = 0;
    let input_amount = 1000;
    let owner = rng.gen();
    let color = Color::default();

    let input = Input::coin(rng.gen(), owner, input_amount, color, 0, maturity, vec![], vec![]);

    let change_output = Output::change(owner, 0, color);
    let withdraw_output = Output::change(rng.gen(), 0, color);

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        vec![Opcode::RET(REG_ONE)].into_iter().collect(),
        vec![],
        vec![input],
        vec![change_output, withdraw_output],
        vec![Witness::default()],
    );

    client.transact(tx);

    let txtor: &Transactor<_> = client.as_ref();
    let outputs = txtor.state_transition().unwrap().tx().outputs();
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
fn base_asset_change_includes_unused_gas_on_revert() {}

// TODO: implement these test cases when TR opcode is supported
#[test]
#[ignore]
fn base_asset_change_is_reduced_by_contract_transfer() {}

#[test]
#[ignore]
fn base_asset_change_is_not_reduced_by_contract_transfer_on_revert() {}

#[test]
#[ignore]
fn asset_change_reduced_by_contract_transfer() {}

#[test]
#[ignore]
fn asset_change_not_reduced_by_contract_transfer_on_revert() {}

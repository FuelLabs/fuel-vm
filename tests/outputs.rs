use fuel_vm::consts::REG_ONE;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Testing of post-execution output handling

#[test]
fn transaction_byte_fees_are_charged_from_base_asset() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
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
        vec![],
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
fn transaction_gas_fees_are_charged_from_base_asset() {
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
        vec![Opcode::ADD(0x12, 0x10, 0x11), Opcode::RET(REG_ONE)]
            .into_iter()
            .collect(),
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
fn base_asset_change_includes_unused_gas() {}

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

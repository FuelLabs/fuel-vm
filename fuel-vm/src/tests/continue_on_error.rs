use fuel_asm::op;
use fuel_types::canonical::Serialize;
use rand::Rng;

use alloc::vec;

use crate::{
    prelude::*,
    tests::test_helpers::{
        assert_panics,
        assert_success,
    },
};

/// If we call a contract that exists but is not in inputs,
/// we catch that, and the execution continues as if the contract was in inputs.
#[test]
fn calling_contract_not_in_inputs_will_collect_and_continue() {
    let mut ctx = TestBuilder::new(2322u64);
    let base_asset_id = ctx.rng.gen();

    // Given
    let contract_code = vec![
        op::movi(0x10, 0x11),
        op::movi(0x11, 0x2a),
        op::add(0x12, 0x10, 0x11),
        op::log(0x10, 0x11, 0x12, 0x00),
        op::ret(0x20),
    ];
    let contract_id = ctx.setup_contract(contract_code, None, None).contract_id;

    let script_code = vec![
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
        op::ret(RegId::ONE),
    ];
    let script_data = Call::new(contract_id, 0, 0).to_bytes();

    // When
    let (res, collected_errors) = ctx
        .start_script(script_code, script_data)
        .max_fee_limit(10_000)
        .gas_price(0)
        .base_asset_id(base_asset_id)
        .coin_input(base_asset_id, 10_000)
        .change_output(base_asset_id)
        .script_gas_limit(10_000)
        .attempt_execute();

    // Then
    assert_success(res.receipts());
    assert_eq!(collected_errors.missing_contract_inputs, vec![contract_id]);
}

#[test]
fn calling_nonexistent_contract_panics_normally() {
    let mut ctx = TestBuilder::new(2322u64);
    let base_asset_id = ctx.rng.gen();

    // Given
    let nonexistent_contract_id = ctx.rng.gen();
    let script_code = vec![
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
        op::ret(RegId::ONE),
    ];
    let script_data = Call::new(nonexistent_contract_id, 0, 0).to_bytes();

    // When
    let (res, collected_errors) = ctx
        .start_script(script_code, script_data)
        .max_fee_limit(10_000)
        .gas_price(0)
        .base_asset_id(base_asset_id)
        .coin_input(base_asset_id, 10_000)
        .change_output(base_asset_id)
        .script_gas_limit(10_000)
        .attempt_execute();

    // Then
    assert_panics(res.receipts(), PanicReason::ContractNotFound);
    assert_eq!(collected_errors.missing_contract_inputs, vec![]);
}

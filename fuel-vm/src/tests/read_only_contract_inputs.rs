//! Tests for read-only contract inputs: contract inputs without the
//! corresponding contract output. Reading the contract's state and balances
//! (and executing its code) is allowed, while any modification panics with
//! [`PanicReason::ContractIsReadOnly`].

use alloc::{
    vec,
    vec::Vec,
};

use crate::{
    prelude::*,
    script_with_data_offset,
    tests::test_helpers::{
        assert_panics,
        assert_success,
    },
    util::test_helpers::TestBuilder,
};
use fuel_asm::{
    GTFArgs,
    RegId,
    op,
};
use fuel_types::{
    AssetId,
    ContractId,
    Immediate18,
    canonical::Serialize,
};

/// Deploys a contract with the given program and calls it (without forwarding
/// any coins) through a read-only contract input.
fn call_read_only_contract(program: Vec<Instruction>) -> Vec<Receipt> {
    call_read_only_contract_with_balance(program, None)
}

/// Same as [`call_read_only_contract`], but the deployed contract starts with
/// the given asset balance.
fn call_read_only_contract_with_balance(
    program: Vec<Instruction>,
    initial_balance: Option<(AssetId, Word)>,
) -> Vec<Receipt> {
    let mut test_context = TestBuilder::new(2322u64);

    let contract_id = test_context
        .setup_contract(program, initial_balance, None)
        .contract_id;

    let (script_call, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );
    let script_call_data = Call::new(contract_id, 0, 0).to_bytes();

    let result = test_context
        .start_script(script_call, script_call_data)
        .script_gas_limit(1_000_000)
        .contract_input(contract_id)
        .fee_input()
        .execute();

    result.receipts().to_vec()
}

#[test]
fn read_only_contract_input__state_read__succeeds() {
    let receipts = call_read_only_contract(vec![
        // Allocate a zeroed 32-byte key
        op::movi(0x15, 32),
        op::aloc(0x15),
        // Read a word from the contract's own state
        op::srw(0x10, 0x11, RegId::HP, 0),
        op::log(0x10, 0x11, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);
}

#[test]
fn read_only_contract_input__balance_read__succeeds() {
    let receipts = call_read_only_contract(vec![
        // Allocate a zeroed 32-byte asset id
        op::movi(0x15, 32),
        op::aloc(0x15),
        // Read the contract's own balance: BAL(dst, asset_ptr, contract_ptr)
        op::bal(0x10, RegId::HP, RegId::FP),
        op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);
}

#[test]
fn read_only_contract_input__state_write__panics() {
    let receipts = call_read_only_contract(vec![
        // Allocate a zeroed 32-byte key
        op::movi(0x15, 32),
        op::aloc(0x15),
        // Attempt to write a word to the contract's own state
        op::sww(RegId::HP, 0x11, RegId::ONE),
        op::ret(RegId::ONE),
    ]);

    assert_panics(&receipts, PanicReason::ContractIsReadOnly);
}

#[test]
fn read_only_contract_input__state_clear__panics() {
    let receipts = call_read_only_contract(vec![
        // Allocate a zeroed 32-byte key
        op::movi(0x15, 32),
        op::aloc(0x15),
        // Attempt to clear a slot of the contract's own state
        op::scwq(RegId::HP, 0x11, RegId::ONE),
        op::ret(RegId::ONE),
    ]);

    assert_panics(&receipts, PanicReason::ContractIsReadOnly);
}

#[test]
fn read_only_contract_input__mint__panics() {
    let receipts = call_read_only_contract(vec![
        // Allocate a zeroed 32-byte sub asset id
        op::movi(0x15, 32),
        op::aloc(0x15),
        // Attempt to mint coins of the contract's own asset
        op::mint(RegId::ONE, RegId::HP),
        op::ret(RegId::ONE),
    ]);

    assert_panics(&receipts, PanicReason::ContractIsReadOnly);
}

#[test]
fn read_only_contract_input__burn__panics() {
    let receipts = call_read_only_contract(vec![
        // Allocate a zeroed 32-byte sub asset id
        op::movi(0x15, 32),
        op::aloc(0x15),
        // Attempt to burn coins of the contract's own asset
        op::burn(RegId::ONE, RegId::HP),
        op::ret(RegId::ONE),
    ]);

    assert_panics(&receipts, PanicReason::ContractIsReadOnly);
}

#[test]
fn read_only_contract_input__zero_amount_mint__succeeds() {
    // Minting zero tokens does not touch the storage, so it is allowed
    // even for a read-only contract.
    let receipts = call_read_only_contract(vec![
        // Allocate a zeroed 32-byte sub asset id
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::mint(RegId::ZERO, RegId::HP),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);
}

#[test]
fn read_only_contract_input__zero_amount_burn__succeeds() {
    // Burning zero tokens does not touch the storage, so it is allowed
    // even for a read-only contract.
    let receipts = call_read_only_contract(vec![
        // Allocate a zeroed 32-byte sub asset id
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::burn(RegId::ZERO, RegId::HP),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);
}

#[test]
fn read_only_contract_input__smo_with_coins__panics() {
    let receipts = call_read_only_contract_with_balance(
        vec![
            // Allocate a zeroed 32-byte recipient address
            op::movi(0x15, 32),
            op::aloc(0x15),
            // Attempt to send a message with coins from the contract's balance
            op::smo(RegId::HP, RegId::HP, RegId::ZERO, RegId::ONE),
            op::ret(RegId::ONE),
        ],
        Some((AssetId::zeroed(), 100)),
    );

    assert_panics(&receipts, PanicReason::ContractIsReadOnly);
}

#[test]
fn read_only_contract_input__transfer_to_contract__panics() {
    let contract_id_ptr = 0x11;
    let asset_id_ptr = 0x12;

    let ops = vec![
        op::gtf_args(contract_id_ptr, RegId::ZERO, GTFArgs::ScriptData),
        op::addi(
            asset_id_ptr,
            contract_id_ptr,
            ContractId::LEN.try_into().unwrap(),
        ),
        // Attempt to transfer coins to the read-only contract
        op::tr(contract_id_ptr, RegId::ONE, asset_id_ptr),
        op::ret(RegId::ONE),
    ];

    let mut test_context = TestBuilder::new(2322u64);
    let asset_id = AssetId::zeroed();

    let contract_id = test_context
        .setup_contract(vec![op::ret(RegId::ONE)], None, None)
        .contract_id;

    let script_data: Vec<u8> = contract_id
        .to_bytes()
        .into_iter()
        .chain(asset_id.to_bytes())
        .collect();

    let result = test_context
        .start_script(ops, script_data)
        .script_gas_limit(1_000_000)
        .contract_input(contract_id)
        .coin_input(asset_id, 100)
        .fee_input()
        .change_output(asset_id)
        .execute();

    assert_panics(result.receipts(), PanicReason::ContractIsReadOnly);
}

#[test]
fn read_only_contract_input__call_with_forwarded_coins__panics() {
    let mut test_context = TestBuilder::new(2322u64);
    let asset_id = AssetId::zeroed();

    let contract_id = test_context
        .setup_contract(vec![op::ret(RegId::ONE)], None, None)
        .contract_id;

    let (script_call, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::movi(0x12, data_offset as Immediate18 + Call::LEN as Immediate18),
            op::movi(0x13, 10),
            // Attempt to forward coins to the read-only contract
            op::call(0x10, 0x13, 0x12, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );
    let script_call_data: Vec<u8> = Call::new(contract_id, 0, 0)
        .to_bytes()
        .into_iter()
        .chain(asset_id.to_bytes())
        .collect();

    let result = test_context
        .start_script(script_call, script_call_data)
        .script_gas_limit(1_000_000)
        .contract_input(contract_id)
        .coin_input(asset_id, 100)
        .fee_input()
        .change_output(asset_id)
        .execute();

    assert_panics(result.receipts(), PanicReason::ContractIsReadOnly);
}

#[test]
fn read_only_contract_input__sibling_writable_contract__can_still_write() {
    let mut test_context = TestBuilder::new(2322u64);

    let writable_id = test_context
        .setup_contract(
            vec![
                // Allocate a zeroed 32-byte key and write to own state
                op::movi(0x15, 32),
                op::aloc(0x15),
                op::sww(RegId::HP, 0x11, RegId::ONE),
                op::ret(RegId::ONE),
            ],
            None,
            None,
        )
        .contract_id;
    let read_only_id = test_context
        .setup_contract(vec![op::ret(RegId::ONE)], None, None)
        .contract_id;

    let (script_call, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );
    let script_call_data = Call::new(writable_id, 0, 0).to_bytes();

    let result = test_context
        .start_script(script_call, script_call_data)
        .script_gas_limit(1_000_000)
        .contract_input(writable_id)
        .contract_input(read_only_id)
        .fee_input()
        .contract_output(&writable_id)
        .execute();

    assert_success(result.receipts());
}

#[test]
fn read_only_contract_input__gtf_output_index__panics() {
    let mut test_context = TestBuilder::new(2322u64);

    let contract_id = test_context
        .setup_contract(vec![op::ret(RegId::ONE)], None, None)
        .contract_id;

    // The contract input is at index 0 and has no corresponding output.
    let ops = vec![
        op::movi(0x11, 0),
        op::gtf(0x10, 0x11, GTFArgs::InputContractOutputIndex as u16),
        op::ret(RegId::ONE),
    ];

    let result = test_context
        .start_script(ops, vec![])
        .script_gas_limit(1_000_000)
        .contract_input(contract_id)
        .fee_input()
        .execute();

    assert_panics(result.receipts(), PanicReason::InputNotFound);
}

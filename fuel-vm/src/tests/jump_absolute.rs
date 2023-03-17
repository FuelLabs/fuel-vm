use fuel_asm::{op, RegId};
use fuel_vm::prelude::*;

#[test]
fn jump_if_not_zero_immediate_jump() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    #[rustfmt::skip]
    let script_jnzi_does_jump = vec![
        op::jnzi(RegId::ONE, 2), // Jump to last instr if reg one is zero
        op::rvrt(RegId::ONE),    // Revert
        op::ret(RegId::ONE),     // Return successfully
    ].into_iter()
    .collect::<Vec<u8>>();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script_jnzi_does_jump,
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate a checked tx");

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Return { .. }));
}

#[test]
fn jump_if_not_zero_immediate_no_jump() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    #[rustfmt::skip]
    let script_jnzi_does_not_jump = vec![
        op::jnzi(RegId::ZERO, 2), // Jump to last instr if reg zero is zero
        op::rvrt(RegId::ONE),     // Revert
        op::ret(RegId::ONE),      // Return successfully
    ].into_iter()
    .collect::<Vec<u8>>();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script_jnzi_does_not_jump,
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate a checked tx");

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Revert { .. }));
}

#[test]
fn jump_dynamic() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    #[rustfmt::skip]
    let script = vec![
        op::movi(RegId::WRITABLE, 3), // Jump target: last instr
        op::jmp(RegId::WRITABLE),     // Jump
        op::rvrt(RegId::ONE),         // Revert
        op::ret(RegId::ONE),          // Return successfully
    ].into_iter()
    .collect::<Vec<u8>>();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, client.gas_costs())
        .expect("failed to generate a checked tx");

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Return { .. }));
}

#[test]
fn jump_dynamic_condition_true() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    #[rustfmt::skip]
    let script = vec![
        op::movi(RegId::WRITABLE, 3),                              // Jump target: last instr
        op::jne(RegId::ZERO, RegId::ONE, RegId::WRITABLE), // Conditional jump (yes, because 0 != 1)
        op::rvrt(RegId::ONE),                                      // Revert
        op::ret(RegId::ONE),                                       // Return successfully
    ].into_iter()
    .collect::<Vec<u8>>();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, client.gas_costs())
        .expect("failed to generate a checked tx");

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Return { .. }));
}

#[test]
fn jump_dynamic_condition_false() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    #[rustfmt::skip]
    let script = vec![
        op::movi(RegId::WRITABLE, 3),                               // Jump target: last instr
        op::jne(RegId::ZERO, RegId::ZERO, RegId::WRITABLE), // Conditional jump (no, because 0 != 0)
        op::rvrt(RegId::ONE),                                       // Revert
        op::ret(RegId::ONE),                                        // Return successfully
    ].into_iter()
    .collect::<Vec<u8>>();

    let tx = Transaction::script(gas_price, gas_limit, maturity, script, vec![], vec![], vec![], vec![])
        .into_checked(height, &params, client.gas_costs())
        .expect("failed to generate a checked tx");

    client.transact(tx);

    let receipts = client.receipts().expect("Expected receipts");

    // Expect the correct receipt
    assert_eq!(receipts.len(), 2);
    assert!(matches!(receipts[0], Receipt::Revert { .. }));
}

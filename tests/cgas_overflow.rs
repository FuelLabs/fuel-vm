use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use fuel_vm::script_with_data_offset;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn cgas_overflow_bug() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_limit = 1_000_000;
    let asset_id: AssetId = rng.gen();
    let call_amount = 500;
    let initial_internal_balance = 1_000_000;

    let mut test_context = TestBuilder::new(2322u64);
    let dest_contract_id = test_context
        .setup_contract(
            vec![
                // log the balance register
                Opcode::RET(REG_BAL),
            ],
            None,
            None,
        )
        .contract_id;

    let program = vec![
        // load amount of tokens
        Opcode::ADDI(0x10, REG_FP, CallFrame::a_offset() as Immediate12),
        Opcode::LW(0x10, 0x10, 0),
        // load asset id
        Opcode::ADDI(0x11, REG_FP, CallFrame::b_offset() as Immediate12),
        Opcode::LW(0x11, 0x11, 0),
        // load contract id
        Opcode::ADDI(0x12, 0x11, 32 as Immediate12),
        Opcode::CALL(0x12, 0x10, 0x11, REG_CGAS),
        Opcode::CALL(0x12, 0x10, 0x11, REG_CGAS),
        Opcode::CALL(0x12, 0x10, 0x11, REG_CGAS),
        Opcode::CALL(0x12, 0x10, 0x11, REG_CGAS),
        Opcode::CALL(0x12, 0x10, 0x11, REG_CGAS),
        Opcode::CALL(0x12, 0x10, 0x11, REG_CGAS),
        Opcode::CALL(0x12, 0x10, 0x11, REG_CGAS),
        Opcode::CALL(0x12, 0x10, 0x11, REG_CGAS),
        Opcode::CALL(0x12, 0x10, 0x11, REG_CGAS),
        Opcode::CALL(0x12, 0x10, 0x11, REG_CGAS),
        Opcode::LOG(REG_CGAS, REG_GGAS, REG_ZERO, REG_ZERO),
        Opcode::RET(REG_BAL),
    ];
    let sender_contract_id = test_context
        .setup_contract(program, Some((asset_id, initial_internal_balance)), None)
        .contract_id;

    let (script_ops, offset) = script_with_data_offset!(
        data_offset,
        vec![
            // load call data to 0x10
            Opcode::MOVI(0x10, data_offset + 64),
            // call the transfer contract
            Opcode::CALL(0x10, REG_ZERO, REG_ZERO, REG_CGAS),
            Opcode::RET(REG_ONE),
        ],
        test_context.tx_offset()
    );
    let script_data: Vec<u8> = [
        asset_id.as_ref(),
        dest_contract_id.as_ref(),
        Call::new(sender_contract_id, call_amount, offset as Word)
            .to_bytes()
            .as_slice(),
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();

    // initiate the call between contracts
    let transfer_tx = test_context
        .gas_limit(gas_limit)
        .gas_price(0)
        .byte_price(0)
        .contract_input(sender_contract_id)
        .contract_input(dest_contract_id)
        .contract_output(&sender_contract_id)
        .contract_output(&dest_contract_id)
        .script(script_ops)
        .script_data(script_data)
        .execute();

    // Ensure transfer tx processed correctly
    assert!(!transfer_tx.should_revert());

    for receipt in transfer_tx.receipts() {
        if let Receipt::Log { ra: cgas, rb: ggas, .. } = receipt {
            assert!(cgas <= ggas, "CGAS exceeded GGAS");
        }
    }
}

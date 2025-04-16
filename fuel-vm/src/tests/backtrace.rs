use alloc::vec;

use crate::{
    consts::*,
    prelude::*,
};
use fuel_asm::{
    RegId,
    op,
};

#[test]
fn backtrace() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;
    let zero_gas_price = 0;

    #[rustfmt::skip]
    let invalid_instruction_bytecode = vec![op::noop()];

    let contract_undefined = test_context
        .setup_contract(invalid_instruction_bytecode, None, None)
        .contract_id;

    #[rustfmt::skip]
    let mut function_call = vec![
        op::movi(0x10,  (contract_undefined.as_ref().len() + WORD_SIZE * 2 + 1) as Immediate18),
        op::aloc(0x10),
    ];

    contract_undefined
        .as_ref()
        .iter()
        .enumerate()
        .for_each(|(i, b)| {
            function_call.push(op::movi(0x10, *b as Immediate18));
            function_call.push(op::sb(RegId::HP, 0x10, 1 + i as Immediate12));
        });

    function_call.push(op::addi(0x10, RegId::HP, 1));
    function_call.push(op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS));
    function_call.push(op::ret(RegId::ONE));

    let contract_call = test_context
        .setup_contract(function_call, None, None)
        .contract_id;

    #[rustfmt::skip]
    let mut script = vec![
        op::movi(0x10, (contract_call.as_ref().len() + WORD_SIZE * 2 + 1) as Immediate18),
        op::aloc(0x10),
    ];

    contract_call
        .as_ref()
        .iter()
        .enumerate()
        .for_each(|(i, b)| {
            script.push(op::movi(0x10, *b as Immediate18));
            script.push(op::sb(RegId::HP, 0x10, i as Immediate12));
        });

    script.push(op::call(RegId::HP, RegId::ZERO, RegId::ZERO, RegId::CGAS));
    script.push(op::ret(RegId::ONE));

    let tx = test_context
        .start_script(script, vec![])
        .script_gas_limit(gas_limit)
        .contract_input(contract_undefined)
        .contract_input(contract_call)
        .fee_input()
        .contract_output(&contract_undefined)
        .contract_output(&contract_call)
        .build();

    let (_, backtrace) = test_context
        .execute_tx_with_backtrace(tx, zero_gas_price)
        .expect("Should execute tx");

    let backtrace = backtrace.expect("Expected erroneous state for undefined opcode");

    assert_eq!(backtrace.contract(), &contract_undefined);

    let id = backtrace.call_stack().last().expect("Caller expected").to();
    assert_eq!(id, &contract_undefined);

    let id = backtrace
        .call_stack()
        .first()
        .expect("Caller expected")
        .to();
    assert_eq!(id, &contract_call);
}

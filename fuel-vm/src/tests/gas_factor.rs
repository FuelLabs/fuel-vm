#![cfg(feature = "std")]

use crate::{
    interpreter::InterpreterParams,
    prelude::*,
};
use core::iter;
use fuel_asm::op;
use fuel_tx::{
    field::Outputs,
    ConsensusParameters,
    FeeParameters,
};

#[test]
fn gas_factor_rounds_correctly() {
    let input = 3_000_000_000;
    let gas_limit = 1_000_000;

    // arbitrary non-negligible primes
    let factor = 5479_f64;
    let gas_price = 6197;

    let gas_costs = GasCosts::default();
    let fee_params = FeeParameters::default().with_gas_price_factor(factor as Word);

    // Random script to consume some gas
    let script = iter::repeat(op::add(0x10, 0x00, 0x01))
        .take(6688)
        .chain(iter::once(op::ret(0x01)))
        .collect();

    let transaction = TestBuilder::new(2322u64)
        .with_fee_params(fee_params)
        .start_script(script, vec![])
        .gas_price(gas_price)
        .script_gas_limit(gas_limit)
        .coin_input(AssetId::default(), input)
        .change_output(AssetId::default())
        .build();

    let fee = TransactionFee::checked_from_tx(
        &gas_costs,
        &fee_params,
        transaction.transaction(),
    )
    .expect("failed to calculate fee");

    let profiler = GasProfiler::default();

    let consensus_params = ConsensusParameters {
        fee_params,
        ..ConsensusParameters::standard()
    };

    let interpreter_params = InterpreterParams::from(&consensus_params);
    let storage = MemoryStorage::default();

    let mut interpreter = Interpreter::<_, _>::with_storage(storage, interpreter_params);
    let gas_costs = interpreter.gas_costs().clone();
    let res = interpreter
        .with_profiler(profiler.clone())
        .transact(transaction)
        .expect("failed to execute transaction");
    let change = res
        .tx()
        .outputs()
        .iter()
        .find_map(|o| match o {
            Output::Change { amount, .. } => Some(amount),
            _ => None,
        })
        .expect("failed to fetch change");

    let initial_balance = input - fee.max_fee();

    let gas_used = profiler.total_gas();

    let refund = res
        .tx()
        .refund_fee(&gas_costs, &fee_params, gas_used)
        .expect("failed to calculate refund");

    assert_eq!(*change, initial_balance + refund);
}

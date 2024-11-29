#![cfg(feature = "std")]

use crate::{
    interpreter::InterpreterParams,
    prelude::*,
};
use core::iter;
use fuel_asm::op;
use fuel_tx::{
    field::{
        MaxFeeLimit,
        Outputs,
    },
    ConsensusParameters,
    FeeParameters,
};

#[test]
fn gas_factor_rounds_correctly() {
    let input = 3_000_000_000;
    let gas_limit = 1_000_000;

    // arbitrary non-negligible primes
    let factor = 5479;
    let gas_price = 6197;

    let large_max_fee_limit = input;

    let gas_costs = GasCosts::default();
    let fee_params = FeeParameters::default().with_gas_price_factor(factor);

    // Random script to consume some gas
    let script = iter::repeat(op::add(0x10, 0x00, 0x01))
        .take(6688)
        .chain(iter::once(op::ret(0x01)))
        .collect();

    let transaction = TestBuilder::new(2322u64)
        .max_fee_limit(large_max_fee_limit)
        .gas_price(gas_price)
        .with_fee_params(fee_params)
        .start_script(script, vec![])
        .script_gas_limit(gas_limit)
        .coin_input(AssetId::default(), input)
        .change_output(AssetId::default())
        .build()
        .into_ready(gas_price, &gas_costs, &fee_params, None)
        .unwrap();

    let profiler = GasProfiler::default();

    let mut consensus_params = ConsensusParameters::standard();
    consensus_params.set_gas_costs(gas_costs);
    consensus_params.set_fee_params(fee_params);

    let interpreter_params = InterpreterParams::new(gas_price, &consensus_params);
    let storage = MemoryStorage::default();

    let mut interpreter = Interpreter::<_, _, _>::with_storage(
        MemoryInstance::new(),
        storage,
        interpreter_params,
    );
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

    let initial_balance = input - res.tx().max_fee_limit();

    let gas_used = profiler.total_gas();

    let refund = res
        .tx()
        .refund_fee(&gas_costs, &fee_params, gas_used, gas_price)
        .expect("failed to calculate refund");

    assert_eq!(*change, initial_balance + refund);
}

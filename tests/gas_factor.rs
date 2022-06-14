use fuel_vm::prelude::*;

use std::iter;

#[test]
fn gas_factor_rounds_correctly() {
    let input = 3_000_000_000;
    let gas_limit = 1_000_000;

    // arbitrary non-negligible primes
    let factor = 5479f64;
    let gas_price = 6197;
    let byte_price = 7451;

    let params = ConsensusParameters::default().with_gas_price_factor(factor as Word);

    // Random script to consume some gas
    let script = iter::repeat(Opcode::ADD(0x10, 0x00, 0x01))
        .take(6688)
        .chain(iter::once(Opcode::RET(0x01)))
        .collect();

    let transaction = TestBuilder::new(2322u64)
        .params(params)
        .gas_price(gas_price)
        .byte_price(byte_price)
        .gas_limit(gas_limit)
        .coin_input(AssetId::default(), input)
        .change_output(AssetId::default())
        .script(script)
        .build();

    let bytes_cost = byte_price * transaction.metered_bytes_size() as Word;
    let bytes_cost = (bytes_cost as f64 / factor).ceil() as Word;

    let gas_limit_cost = gas_price * gas_limit;
    let gas_limit_cost = (gas_limit_cost as f64 / factor).ceil() as Word;

    let profiler = GasProfiler::default();

    let change = Interpreter::with_memory_storage()
        .with_params(params)
        .with_profiler(profiler.clone())
        .transact(transaction)
        .expect("failed to execute transaction")
        .tx()
        .outputs()
        .iter()
        .find_map(|o| match o {
            Output::Change { amount, .. } => Some(*amount),
            _ => None,
        })
        .expect("failed to fetch change");

    let initial_balance = input - gas_limit_cost - bytes_cost;

    let gas_used = profiler.total_gas();

    let gas_remainder = gas_limit - gas_used;
    let gas_remainder = gas_price * gas_remainder;
    let gas_remainder = (gas_remainder as f64 / factor).floor() as Word;

    assert_eq!(change, initial_balance + gas_remainder);
}

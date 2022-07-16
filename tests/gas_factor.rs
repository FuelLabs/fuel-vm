use fuel_vm::prelude::*;

use std::iter;

#[test]
fn gas_factor_rounds_correctly() {
    let input = 3_000_000_000;
    let gas_limit = 1_000_000;

    // arbitrary non-negligible primes
    let factor = 5479f64;
    let gas_price = 6197;

    let params = ConsensusParameters::default().with_gas_price_factor(factor as Word);

    // Random script to consume some gas
    let script = iter::repeat(Opcode::ADD(0x10, 0x00, 0x01))
        .take(6688)
        .chain(iter::once(Opcode::RET(0x01)))
        .collect();

    let transaction = TestBuilder::new(2322u64)
        .start_script(script, vec![])
        .params(params)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .coin_input(AssetId::default(), input)
        .change_output(AssetId::default())
        .build();

    let fee = TransactionFee::checked_from_tx(&params, transaction.transaction()).expect("failed to calculate fee");

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

    let initial_balance = input - fee.total();

    let gas_used = profiler.total_gas();

    let gas_remainder = gas_limit - gas_used;
    let refund =
        TransactionFee::gas_refund_value(&params, gas_remainder, gas_price).expect("failed to calculate refund");

    assert_eq!(change, initial_balance + refund);
}

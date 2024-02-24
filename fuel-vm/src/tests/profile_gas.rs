#![cfg(feature = "std")]

use fuel_asm::{
    op,
    RegId,
};
use fuel_tx::TransactionBuilder;
use fuel_vm::prelude::*;
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};

#[test]
fn profile_gas() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_limit = 1_000;
    let arb_fee_limit = 2_000;
    let maturity = Default::default();
    let height = Default::default();

    // Deploy contract with loops
    let reg_a = 0x20;

    let case_out_of_gas = 1_000;
    let mut rounds = [2, 12, 22, case_out_of_gas].into_iter().map(|count| {
        let script_code = vec![
            op::xor(reg_a, reg_a, reg_a),    // r[a] := 0
            op::ori(reg_a, reg_a, count),    // r[a] := count
            op::subi(reg_a, reg_a, 1),       // r[a] -= count  <-|
            op::jnei(RegId::ZERO, reg_a, 2), // Jump to ---------|
            op::ret(RegId::ONE),
        ];

        let tx_deploy =
            TransactionBuilder::script(script_code.into_iter().collect(), vec![])
                .max_fee_limit(arb_fee_limit)
                .add_unsigned_coin_input(
                    SecretKey::random(rng),
                    rng.gen(),
                    arb_fee_limit,
                    Default::default(),
                    rng.gen(),
                )
                .script_gas_limit(gas_limit)
                .maturity(maturity)
                .finalize_checked(height);

        let output = GasProfiler::default();

        let mut client = MemoryClient::from_txtor(
            Interpreter::<_, _>::with_memory_storage()
                .with_profiler(output.clone())
                .build()
                .into(),
        );

        let receipts = client.transact(tx_deploy);

        if let Some(Receipt::ScriptResult { result, .. }) = receipts.last() {
            if count == case_out_of_gas {
                assert!(
                    !matches!(result, &ScriptExecutionResult::Success),
                    "Expected out-of-gas error, got success"
                );
                let panic_reason = receipts
                    .iter()
                    .find_map(Receipt::reason)
                    .map(|r| *r.reason())
                    .expect("Expected a panic reason.");
                assert!(
                    matches!(panic_reason, PanicReason::OutOfGas),
                    "Expected out-of-gas error, got {panic_reason:?}"
                );
            } else {
                matches!(result, &ScriptExecutionResult::Success);
            }
        } else {
            panic!("Missing result receipt");
        }

        output.data().expect("failed to fetch profiling data")
    });

    let round0 = rounds.next().unwrap();
    let round1 = rounds.next().unwrap();
    let round2 = rounds.next().unwrap();

    let round_out_of_gas = rounds.next().unwrap();
    let out_of_gas = round_out_of_gas.gas();
    assert_eq!(out_of_gas.values().sum::<u64>(), gas_limit);

    let mut keys: Vec<_> = round0.gas().keys().collect();
    keys.sort();

    let items0: Vec<_> = keys.iter().map(|k| round0.gas().get(k)).collect();
    let items1: Vec<_> = keys.iter().map(|k| round1.gas().get(k)).collect();
    let items2: Vec<_> = keys.iter().map(|k| round2.gas().get(k)).collect();

    // Non-looped instructions should have same gas count
    assert!(items0[0] == items1[0] && items0[0] == items2[0]);
    assert!(items0[1] == items1[1] && items0[1] == items2[1]);
}

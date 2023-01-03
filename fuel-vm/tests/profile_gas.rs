use fuel_tx::TransactionBuilder;
use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn profile_gas() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_price = 1;
    let gas_limit = 1_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    // Deploy contract with loops
    let reg_a = 0x20;

    let case_out_of_gas = 1_000;
    let mut rounds = [2, 12, 22, case_out_of_gas].into_iter().map(|count| {
        let script_code: Vec<Opcode> = vec![
            Opcode::XOR(reg_a, reg_a, reg_a), // r[a] := 0
            Opcode::ORI(reg_a, reg_a, count), // r[a] := count
            Opcode::SUBI(reg_a, reg_a, 1),    // r[a] -= count  <-|
            Opcode::JNEI(REG_ZERO, reg_a, 2), // Jump to ---------|
            Opcode::RET(REG_ONE),
        ];

        let tx_deploy = TransactionBuilder::script(script_code.into_iter().collect(), vec![])
            .add_unsigned_coin_input(rng.gen(), rng.gen(), 1, Default::default(), rng.gen(), 0)
            .gas_limit(gas_limit)
            .gas_price(gas_price)
            .maturity(maturity)
            .finalize_checked(height, &params);

        let output = GasProfiler::default();

        let mut client = MemoryClient::from_txtor(
            Interpreter::with_memory_storage()
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
                    "Expected out-of-gas error, got {:?}",
                    panic_reason
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

use std::sync::{Arc, Mutex};

use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use fuel_vm::profiler::{ProfileReceiver, ProfilingData};

#[test]
fn profile_gas() {
    let gas_price = 1;
    let gas_limit = 1_000;
    let byte_price = 0;
    let maturity = 0;

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

        let tx_deploy = Transaction::script(
            gas_price,
            gas_limit,
            byte_price,
            maturity,
            script_code.into_iter().collect(),
            vec![],
            vec![],
            vec![],
            vec![],
        );

        #[derive(Clone)]
        struct ProfilingOutput {
            data: Arc<Mutex<Option<ProfilingData>>>,
        }

        impl ProfileReceiver for ProfilingOutput {
            fn on_transaction(&mut self, _state: &Result<ProgramState, InterpreterError>, data: &ProfilingData) {
                let mut guard = self.data.lock().unwrap();
                *guard = Some(data.clone());
            }
        }

        let output = ProfilingOutput {
            data: Arc::new(Mutex::new(None)),
        };

        let mut client = MemoryClient::from_txtor(
            Interpreter::with_memory_storage()
                .with_profiling(Box::new(output.clone()))
                .into(),
        );

        let receipts = client.transact(tx_deploy);

        if let Some(Receipt::ScriptResult { result, .. }) = receipts.last() {
            if count == case_out_of_gas {
                assert!(!result.is_success(), "Expected out-of-gas error, got success");
                assert!(
                    matches!(result.reason(), PanicReason::OutOfGas),
                    "Expected out-of-gas error, got {:?}",
                    result.reason()
                );
            } else {
                assert!(result.is_success());
            }
        } else {
            panic!("Missing result receipt");
        }

        let guard = output.data.lock().unwrap();
        guard.as_ref().unwrap().clone()
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

    // Gas cost for looped instructions should increase linearly
    assert!(items0[2] < items1[2] && items1[2] < items2[2]);
    assert_eq!(items2[2] - items1[2], items1[2] - items0[2]);
    assert_eq!(items2[3] - items1[3], items1[3] - items0[3]);
}

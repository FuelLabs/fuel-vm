use fuel_asm::{op, RegId};
use fuel_tx::{ScriptExecutionResult, TransactionBuilder};

use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use std::sync::{Arc, Mutex};

const HALF_WORD_SIZE: u64 = (WORD_SIZE as u64) / 2;

#[test]
fn code_coverage() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_price = 1;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    // Deploy contract with loops
    let reg_a = 0x20;

    let script_code = vec![
        op::jnei(RegId::ZERO, RegId::ONE, 2),  // Skip next
        op::xor(reg_a, reg_a, reg_a),          // Skipped
        op::jnei(RegId::ZERO, RegId::ZERO, 2), // Do not skip
        op::xor(reg_a, reg_a, reg_a),          // Executed
        op::ret(RegId::ONE),
    ];

    let tx_script = TransactionBuilder::script(script_code.into_iter().collect(), vec![])
        .add_unsigned_coin_input(rng.gen(), rng.gen(), 1, Default::default(), rng.gen(), 0)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(maturity)
        .finalize_checked(height, &params, &GasCosts::default());

    #[derive(Clone, Default)]
    struct ProfilingOutput {
        data: Arc<Mutex<Option<ProfilingData>>>,
    }

    impl ProfileReceiver for ProfilingOutput {
        fn on_transaction(&mut self, _state: &Result<ProgramState, InterpreterError>, data: &ProfilingData) {
            let mut guard = self.data.lock().unwrap();
            *guard = Some(data.clone());
        }
    }

    let output = ProfilingOutput::default();

    let mut client = MemoryClient::from_txtor(
        Interpreter::with_memory_storage()
            .with_profiler(output.clone())
            .build()
            .into(),
    );

    let receipts = client.transact(tx_script);

    if let Some(Receipt::ScriptResult { result, .. }) = receipts.last() {
        assert!(matches!(result, ScriptExecutionResult::Success));
    } else {
        panic!("Missing result receipt");
    }

    let guard = output.data.lock().unwrap();
    let case = guard.as_ref().unwrap().clone();

    let mut items: Vec<_> = case.coverage().iter().collect();
    items.sort();

    let expect = vec![0, 2, 3, 4];

    assert_eq!(items.len(), expect.len());

    for (item, expect) in items.into_iter().zip(expect.into_iter()) {
        assert_eq!(*item, InstructionLocation::new(None, expect * HALF_WORD_SIZE));
    }
}

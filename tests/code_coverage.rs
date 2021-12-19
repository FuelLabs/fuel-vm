use std::sync::{Arc, Mutex};

use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use fuel_vm::profiler::{InstructionLocation, ProfileReceiver, ProfilingData};

#[test]
fn code_coverage() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let salt: Salt = rng.gen();

    let gas_price = 1;
    let gas_limit = 1_000;
    let maturity = 0;

    // Deploy contract with loops
    let reg_a = 0x20;

    let contract_code: Vec<Opcode> = vec![
        Opcode::JNEI(REG_ZERO, REG_ONE, 2),  // Skip next
        Opcode::XOR(reg_a, reg_a, reg_a),    // Skipped
        Opcode::JNEI(REG_ZERO, REG_ZERO, 2), // Do not skip
        Opcode::XOR(reg_a, reg_a, reg_a),    // Executed
        Opcode::RET(REG_ONE),
    ];

    let program: Witness = contract_code.clone().into_iter().collect::<Vec<u8>>().into();
    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let contract_id = contract.id(&salt, &contract_root);

    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), contract_id);
    let output = Output::contract(0, rng.gen(), rng.gen());

    let tx_deploy = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        contract_code.clone().into_iter().collect(),
        vec![],
        vec![input],
        vec![output],
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
        assert!(result.is_success());
    } else {
        panic!("Missing result receipt");
    }

    let guard = output.data.lock().unwrap();
    let case = guard.as_ref().unwrap().clone();

    let mut items: Vec<_> = case.coverage().iter().collect();
    items.sort();

    let expect = vec![0, 2, 3, 4];

    assert_eq!(items.len(), expect.len());

    println!("{:?}", items);

    for (item, expect) in items.into_iter().zip(expect.into_iter()) {
        assert_eq!(*item, InstructionLocation::new(None, expect * 4));
    }
}

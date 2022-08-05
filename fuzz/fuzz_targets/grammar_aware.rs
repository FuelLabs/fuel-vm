#![feature(bench_black_box)]
#![no_main]

use std::hint::black_box;

use libfuzzer_sys::fuzz_target;

use fuel_vm::prelude::*;
use fuzzed_ops::*;

mod fuzzed_ops;

type FuzzProgram = Vec<FuzzedOp>;

#[derive(arbitrary::Arbitrary, Debug)]
struct FuzzData {
    program: FuzzProgram,
    script_data: Vec<u8>,
}

fuzz_target!(|data: FuzzData| {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    let opcodes: Vec<Opcode> = data.program.iter().map(|&fuzzed_op| Opcode::from(fuzzed_op)).collect();
    let script: Vec<u8> = opcodes.iter().copied().collect();

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        data.script_data,
        vec![],
        vec![],
        vec![],
    )
    .check(height, &params)
    .expect("failed to generate a checked tx");

    drop(black_box(client.transact(tx)));
});

#![feature(bench_black_box)]
#![no_main]

use std::hint::black_box;

use libfuzzer_sys::fuzz_target;

use fuel_vm::prelude::*;

#[derive(arbitrary::Arbitrary, Debug)]
struct FuzzData {
    program: Vec<Opcode>,
    script_data: Vec<u8>,
}

fuzz_target!(|data: FuzzData| {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000;
    let maturity = Default::default();
    let height = Default::default();
    let params = ConsensusParameters::DEFAULT;

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        data.program.iter().copied().collect(),
        data.script_data,
        vec![],
        vec![],
        vec![],
    )
    .into_checked(height, &params)
    .expect("failed to generate a checked tx");

    drop(black_box(client.transact(tx)));
});

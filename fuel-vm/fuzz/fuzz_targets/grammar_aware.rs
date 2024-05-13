#![feature(bench_black_box)]
#![no_main]

use std::hint::black_box;

use libfuzzer_sys::fuzz_target;
use fuel_vm::fuel_tx::field::MaxFeeLimit;

use fuel_vm::prelude::*;
use fuel_vm::prelude::policies::Policies;

#[derive(arbitrary::Arbitrary, Debug)]
struct FuzzData {
    program: Vec<Opcode>,
    script_data: Vec<u8>,
}

fuzz_target!(|data: FuzzData| {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000;
    let height = Default::default();
    let params = ConsensusParameters::standard();

    let mut tx = Transaction::script(
        gas_limit,
        data.program.iter().copied().map(|op| op as u8).collect::<Vec<u8>>(),
        data.script_data,
        Policies::new(),
        vec![],
        vec![],
        vec![],
    );

    tx.set_max_fee_limit(1_000);

    let tx = tx.into_checked(height, &params).expect("failed to generate a checked tx");

    drop(black_box(client.transact(tx)));
});

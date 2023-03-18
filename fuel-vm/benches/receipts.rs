use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use fuel_asm::{op, RegId};
use fuel_tx::{ConsensusParameters, Finalizable, Script, TransactionBuilder};
use fuel_vm::checked_transaction::Checked;
use fuel_vm::gas::GasCosts;
use fuel_vm::prelude::{Interpreter, IntoChecked, MemoryStorage};
use std::iter::once;

fn receipts(c: &mut Criterion) {
    let log_amounts = [1usize, 10usize, 100, 1_000, 10_000, 100_000];

    let mut group = c.benchmark_group("receipt roots");
    for logs in log_amounts {
        group.throughput(Throughput::Elements(logs as u64));
        group.bench_with_input(BenchmarkId::new("bmt", logs), &logs, |b, &logs| {
            b.iter(|| entrypoint(false, logs));
        });
        group.bench_with_input(BenchmarkId::new("accumulator", logs), &logs, |b, &logs| {
            b.iter(|| entrypoint(true, logs));
        });
    }
    group.finish();
}

fn entrypoint(receipt_accumulator: bool, logs: usize) {
    let params = ConsensusParameters {
        receipt_accumulator,
        ..ConsensusParameters::DEFAULT
    };
    let tx = new_tx(logs);
    process_tx(black_box(tx), params)
}

fn new_tx(logs: usize) -> Checked<Script> {
    let script: Vec<u8> = (0..logs)
        .map(|_| op::log(RegId::ONE, RegId::ONE, RegId::ONE, RegId::ONE))
        .chain(once(op::ret(RegId::ZERO)))
        .collect();

    TransactionBuilder::script(script, vec![])
        .gas_limit(10000000)
        .finalize()
        .into_checked(0, &ConsensusParameters::DEFAULT, &GasCosts::default())
        .unwrap()
}

fn process_tx(tx: Checked<Script>, params: ConsensusParameters) {
    let mut interpreter = Interpreter::<_, _>::with_storage(MemoryStorage::default(), params, GasCosts::default());
    let result = interpreter.transact(tx).unwrap();
    if result.should_revert() {
        panic!("test failed {:?}", result)
    }
}

criterion_group!(benches, receipts);
criterion_main!(benches);

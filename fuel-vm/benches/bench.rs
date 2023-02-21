use criterion::{criterion_group, criterion_main, Criterion};
use fuel_vm::{
    constraints::reg_key::benchmarks::bench_split_reg,
    interpreter::{bench_alu_clear, bench_prepare_call, bench_set_receipt_root, bench_sha256},
};

pub fn criterion_benchmark(c: &mut Criterion) {
    bench_prepare_call(c);
    bench_split_reg(c);
    bench_alu_clear(c);
    bench_set_receipt_root(c);
    bench_sha256(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

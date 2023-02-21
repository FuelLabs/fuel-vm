use criterion::{criterion_group, criterion_main, Criterion};
use fuel_vm::{constraints::reg_key::benchmarks::bench_split_reg, interpreter::bench_prepare_call};

pub fn criterion_benchmark(c: &mut Criterion) {
    bench_prepare_call(c);
    bench_split_reg(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
